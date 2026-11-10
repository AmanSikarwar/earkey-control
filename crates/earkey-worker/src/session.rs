//! A supervised passthrough session: the worker child and the bounded supervisor diagnostic.
//!
//! The worker repeats the pause and `stay-inactive` gate check before opening
//! anything, acquires the verified node read-only, creates the proxy, waits for
//! Android readiness, reports startup and loop progress as supervisor frames,
//! and forwards until stopped. The supervisor runs the state machine over the
//! process adapter, closes the gate at the session deadline, and measures the
//! orderly stop. Neither writes policy.

use crate::check::{self, Report};
use crate::environment::Observed;
use crate::forward::{self, End};
use crate::proxy::{Proxy, wait_ready};
use crate::proxy_check::proxy_keys;
use earkey_supervisor::process::{Adapter, Host, progress_frame, started_frame};
use earkey_supervisor::{Action, Gate, InactiveReason, Limits, Phase, StopCause, Supervisor};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// Initial production deadlines, chosen from the drill and proxy measurements:
/// Android opened the proxy about 7 s after creation at early boot, pause took
/// 105 ms and the forced-kill path about 1 s on the reference phone.
pub const LIMITS: Limits = Limits {
    start: Duration::from_secs(20),
    progress: Duration::from_secs(1),
    pause: Duration::from_secs(1),
    kill: Duration::from_secs(2),
    restart: Duration::from_secs(5),
};
/// Forwarding bounds inside the worker. The session limit exceeds any supervised diagnostic.
pub const FORWARD: forward::Limits = forward::Limits {
    stall: Duration::from_millis(250),
    session: Duration::from_secs(900),
    tick: Duration::from_millis(100),
};
const READINESS: Duration = Duration::from_secs(15);
const TICK: Duration = Duration::from_millis(50);
/// Supervised diagnostic length bounds, in seconds.
pub const SESSION_SECONDS: std::ops::RangeInclusive<u64> = 10..=600;

/// Worker child: returns true only for an orderly stop after a full acquisition.
pub fn worker(
    root: &Path,
    environment: &Observed,
    gate: impl FnOnce() -> io::Result<Gate>,
    stop: &AtomicBool,
    frames: &mut impl Write,
    report: &mut impl Write,
) -> bool {
    let line = |report: &mut dyn Write, key: &str, value: &dyn std::fmt::Display| {
        let _ = writeln!(report, "worker_{key}={value}");
    };
    match gate() {
        Ok(gate) if gate == Gate::OPEN => {}
        Ok(_) => {
            line(report, "gate", &"closed");
            return false;
        }
        Err(error) => {
            line(report, "gate", &format!("error:{:?}", error.kind()));
            return false;
        }
    }
    let (checked, opened): (Report, _) = check::acquire(root, environment);
    for (key, value) in &checked.lines {
        line(report, key, value);
    }
    let (Some(verified), Some(opened)) = (checked.checked, opened) else {
        return false;
    };
    let dev = root.join("dev");
    let proxy = match Proxy::create(&dev, verified.input_id, &proxy_keys(verified.evidence)) {
        Ok(proxy) => proxy,
        Err(error) => {
            line(report, "proxy", &format!("error:{:?}", error.kind()));
            return false;
        }
    };
    let ready = match wait_ready(
        &root.join("sys"),
        &dev,
        &root.join("proc"),
        &proxy.sysname,
        READINESS,
    ) {
        Ok(ready) => ready,
        Err(error) => {
            line(report, "proxy_ready", &format!("error:{:?}", error.kind()));
            return false;
        }
    };
    line(report, "proxy_ready_ms", &ready.elapsed.as_millis());
    if frames
        .write_all(&started_frame())
        .and_then(|()| frames.flush())
        .is_err()
    {
        return false;
    }
    let mut last = 0;
    let mut tally = forward::Tally::default();
    let end = forward::run(
        &opened,
        &proxy,
        verified.evidence,
        &[],
        FORWARD,
        stop,
        &mut tally,
        |counter| {
            last = counter;
            let _ = frames
                .write_all(&progress_frame(counter))
                .and_then(|()| frames.flush());
        },
    );
    line(report, "progress", &last);
    line(report, "read_keys", &tally.read.summary());
    line(report, "forwarded_keys", &tally.forwarded.summary());
    line(report, "cleanup_releases", &tally.cleanup_releases);
    match end {
        Ok(end) => {
            line(report, "end", &format!("{end:?}"));
            end == End::Stopped
        }
        Err(error) => {
            line(report, "end", &format!("error:{:?}", error.kind()));
            false
        }
    }
}

struct SessionHost<G> {
    gate: G,
    exe: PathBuf,
    args: Vec<String>,
    end_at: Instant,
}

impl<G: FnMut() -> io::Result<Gate>> Host for SessionHost<G> {
    fn gate(&mut self) -> io::Result<Gate> {
        let mut gate = (self.gate)()?;
        gate.paused |= Instant::now() >= self.end_at;
        Ok(gate)
    }

    fn command(&mut self) -> Command {
        let mut command = Command::new(&self.exe);
        command.args(&self.args);
        command
    }

    fn persist_latch(&mut self) -> io::Result<()> {
        Err(io::Error::other("diagnostic_does_not_persist"))
    }
}

/// Supervises `exe args` for `session`, then pauses it and reports the stop.
///
/// # Safety
/// As for [`Adapter::new`]: nothing else may reap this process's children.
pub unsafe fn supervise(
    exe: PathBuf,
    args: Vec<String>,
    session: Duration,
    limits: Limits,
    gate: impl FnMut() -> io::Result<Gate>,
) -> Report {
    let mut report = Report::default();
    let started = Instant::now();
    let end_at = started + session;
    let mut host = SessionHost {
        gate,
        exe,
        args,
        end_at,
    };
    let mut supervisor = match Supervisor::new(limits) {
        Ok(supervisor) => supervisor,
        Err(error) => return fail(report, "supervisor", format!("{error:?}")),
    };
    // SAFETY: Forwarded from the caller's contract.
    let mut adapter = unsafe { Adapter::new() };
    let hard_end = end_at + limits.pause + limits.kill + limits.start;
    let mut actions = Vec::new();
    let mut running_at = None;
    let mut timeline = Vec::new();
    let mut last_phase = None;
    report
        .lines
        .push(("supervisor_started_uptime_ms", uptime_ms()));
    loop {
        let action = match adapter.step(&mut supervisor, &mut host, TICK) {
            Ok(action) => action,
            Err(error) => {
                let status = supervisor.status();
                timeline.push(format!(
                    "{}:latch_write_refused:{:?}",
                    started.elapsed().as_millis(),
                    error.kind()
                ));
                report
                    .lines
                    .push(("supervisor_actions", format!("{actions:?}")));
                report
                    .lines
                    .push(("supervisor_timeline", timeline.join("; ")));
                report
                    .lines
                    .push(("supervisor_phase", format!("{:?}", status.phase)));
                report
                    .lines
                    .push(("supervisor_failures", status.failures.to_string()));
                report.lines.push((
                    "supervisor_last_fault",
                    status
                        .last_fault
                        .map_or("none".into(), |fault| format!("{fault:?}")),
                ));
                return fail(
                    report,
                    "supervisor_latch",
                    format!("error:{:?}", error.kind()),
                );
            }
        };
        actions.extend(action);
        let phase = supervisor.status().phase;
        let at = started.elapsed().as_millis();
        if let Some(action) = action {
            timeline.push(format!("{at}:{action:?}"));
        }
        if last_phase != Some(phase) {
            timeline.push(format!("{at}:{phase:?}"));
            last_phase = Some(phase);
        }
        if running_at.is_none() && phase == Phase::Running {
            running_at = Some(started.elapsed());
        }
        let now = Instant::now();
        if now >= end_at && phase == Phase::Inactive(InactiveReason::Paused) {
            report
                .lines
                .push(("supervisor_stop_ms", (now - end_at).as_millis().to_string()));
            break;
        }
        if matches!(phase, Phase::Latched | Phase::Unsafe(_)) || now >= hard_end {
            break;
        }
    }
    let status = supervisor.status();
    report.lines.push((
        "supervisor_running_after_ms",
        running_at.map_or("never".into(), |at| at.as_millis().to_string()),
    ));
    report
        .lines
        .push(("supervisor_actions", format!("{actions:?}")));
    report
        .lines
        .push(("supervisor_timeline", timeline.join("; ")));
    report
        .lines
        .push(("supervisor_phase", format!("{:?}", status.phase)));
    report
        .lines
        .push(("supervisor_failures", status.failures.to_string()));
    report.lines.push((
        "supervisor_last_fault",
        status
            .last_fault
            .map_or("none".into(), |fault| format!("{fault:?}")),
    ));
    report.passed = running_at.is_some()
        && status.failures == 0
        && status.phase == Phase::Inactive(InactiveReason::Paused)
        && actions == [Action::Start, Action::Stop(StopCause::Paused)];
    report.lines.push((
        "supervise",
        if report.passed { "pass" } else { "fail" }.into(),
    ));
    report
}

/// Milliseconds since boot from `/proc/uptime`, for correlating with external events.
fn uptime_ms() -> String {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse::<f64>().ok())
        .map_or("unavailable".into(), |seconds| {
            format!("{:.0}", seconds * 1000.0)
        })
}

fn fail(mut report: Report, key: &'static str, value: String) -> Report {
    report.lines.push((key, value));
    report.lines.push(("supervise", "fail".into()));
    report
}
