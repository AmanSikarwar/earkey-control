//! Production worker loop driven by the durable policy.
//!
//! The loop reports progress on every pass, including while Android has not yet
//! opened the proxy. It waits without failing while the
//! policy is disabled or unreadable, no enrolled profile matches the running
//! ROM and kernel, the accessory is absent, a key is down at
//! acquisition, or another program holds a grab. It forwards with the policy's
//! blocked groups, returns to waiting on unplug, and reacquires at the next
//! neutral boundary when the saved generation changes. Only `SIGTERM` (the stop
//! flag) ends it successfully; any other session fault ends it with failure so
//! the supervisor counts it.

use crate::device::Opened;
use crate::environment::Observed;
use crate::forward::{self, End, Tally};
use crate::inventory::{self, Observation};
use crate::layout;
use crate::proxy::{Proxy, wait_ready};
use crate::proxy_check::proxy_keys;
use earkey_engine::Fault;
use earkey_policy::identity::{Evidence, Group};
use earkey_policy::policy::{Policy, RecoveryLatch};
use earkey_policy::store::{EnrollmentRecord, Runtime, Store};
use earkey_supervisor::process::{progress_frame, started_frame};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const READY_POLL: Duration = Duration::from_millis(50);

/// Inputs for one worker process.
pub struct Context<'a> {
    pub root: &'a Path,
    pub environment: &'a Observed,
    /// `Generic.kl` digest; production computes it once with [`layout::generic_digest`].
    pub layout: &'a str,
    pub store: &'a Store,
    /// Wait between passes while inactive, and the policy watch interval.
    pub poll: Duration,
    pub readiness: Duration,
    pub forward: forward::Limits,
}

struct Acquired {
    opened: Opened,
    evidence: Evidence,
    groups: Vec<Group>,
    generation: u64,
}

fn load(store: &Store) -> Result<Policy, String> {
    let records = store
        .enrollments()
        .map_err(|error| format!("enrollments:{error:?}"))?;
    let verified: Vec<_> = records
        .iter()
        .filter_map(EnrollmentRecord::verified)
        .collect();
    store
        .load(&verified)
        .map_err(|error| format!("policy:{error:?}"))
}

fn digests<'a>(root: &Path, nodes: &[Observation], generic: &'a str) -> Vec<&'a str> {
    nodes
        .iter()
        .map(|node| {
            if layout::has_override(root, node) {
                "override"
            } else {
                generic
            }
        })
        .collect()
}

/// One acquisition attempt, or the state to wait in.
fn acquire(ctx: &Context<'_>) -> Result<Acquired, String> {
    let policy = load(ctx.store)?;
    if !policy.enabled() {
        return Err("disabled".into());
    }
    let recovery = policy.recovery();
    if recovery.paused || recovery.boot_disabled || recovery.latch != RecoveryLatch::Clear {
        return Err("recovery_inactive".into());
    }
    let env = ctx.environment.as_environment();
    if !(0..policy.profile_count())
        .filter_map(|index| policy.profile_evidence(index))
        .any(|evidence| evidence.matches_environment(&env))
    {
        return Err("unsupported_environment".into());
    }
    let (sys, dev) = (ctx.root.join("sys"), ctx.root.join("dev"));
    let nodes =
        inventory::scan(&sys, &dev).map_err(|error| format!("inventory:{:?}", error.kind()))?;
    let marks = digests(ctx.root, &nodes, ctx.layout);
    let candidates: Vec<_> = nodes
        .iter()
        .zip(&marks)
        .map(|(n, d)| n.candidate(d))
        .collect();
    let targets = policy
        .targets(&env, &candidates)
        .map_err(|error| format!("refused:{error:?}"))?;
    // ponytail: the first target only; several enrolled accessories at once is a later case.
    let target = targets
        .first()
        .ok_or_else(|| "waiting_accessory".to_owned())?;
    let index = target.profile_index();
    let evidence = policy
        .profile_evidence(index)
        .ok_or_else(|| "profile_missing".to_owned())?;
    let node = nodes
        .iter()
        .find(|node| node.instance == target.instance())
        .ok_or_else(|| "target_vanished".to_owned())?;
    let opened = Opened::open(&dev, node).map_err(|error| format!("open:{:?}", error.kind()))?;
    let fresh =
        inventory::scan(&sys, &dev).map_err(|error| format!("rescan:{:?}", error.kind()))?;
    let fresh_marks = digests(ctx.root, &fresh, ctx.layout);
    let fresh: Vec<_> = fresh
        .iter()
        .zip(&fresh_marks)
        .map(|(n, d)| n.candidate(d))
        .collect();
    match policy.revalidate(target, &opened.candidate(node, ctx.layout), &env, &fresh) {
        Ok(true) => {}
        Ok(false) => return Err("revalidate:mismatch".into()),
        Err(error) => return Err(format!("revalidate:{error:?}")),
    }
    if !opened
        .pressed_keys()
        .map_err(|error| format!("neutral:{:?}", error.kind()))?
        .is_empty()
    {
        return Err("not_neutral".into());
    }
    Ok(Acquired {
        opened,
        evidence,
        groups: policy
            .blocked_controls(index)
            .map(|control| control.group)
            .collect(),
        generation: policy.generation(),
    })
}

/// `worker.status` text: the state and the generation actually in force (0 when none).
pub fn status_text(state: &str) -> String {
    let active = state
        .strip_prefix("active:generation=")
        .and_then(|rest| rest.split(':').next())
        .unwrap_or("0");
    format!("state={state}\nactive_generation={active}\n")
}

/// Runs until the stop flag ends a session or an unrecoverable fault occurs.
pub fn run(
    ctx: &Context<'_>,
    stop: &AtomicBool,
    frames: &mut impl Write,
    report: &mut impl Write,
) -> bool {
    if ctx
        .store
        .write_runtime(Runtime::Worker, &status_text("discovering"))
        .is_err()
        || frames
            .write_all(&started_frame())
            .and_then(|()| frames.flush())
            .is_err()
    {
        return false;
    }
    let active = AtomicU64::new(0);
    let newer_generation = AtomicU64::new(0);
    let done = AtomicBool::new(false);
    thread::scope(|scope| {
        scope.spawn(|| {
            while !done.load(Ordering::SeqCst) {
                thread::sleep(ctx.poll);
                let generation = active.load(Ordering::SeqCst);
                if generation != 0
                    && let Ok(policy) = load(ctx.store)
                    && policy.generation() > generation
                {
                    newer_generation.store(policy.generation(), Ordering::SeqCst);
                }
            }
        });
        let result = pump(ctx, stop, &newer_generation, &active, frames, report);
        done.store(true, Ordering::SeqCst);
        result
    })
}

fn pump(
    ctx: &Context<'_>,
    stop: &AtomicBool,
    newer_generation: &AtomicU64,
    active: &AtomicU64,
    frames: &mut impl Write,
    report: &mut impl Write,
) -> bool {
    let mut counter = 0u64;
    let mut state = "discovering".to_owned();
    let mut note = |report: &mut dyn Write, value: String| {
        if value != state {
            let _ = writeln!(report, "worker_state={value}");
            let _ = ctx
                .store
                .write_runtime(Runtime::Worker, &status_text(&value));
            state = value;
        }
    };
    loop {
        if stop.load(Ordering::SeqCst) {
            note(report, "stopped".into());
            return true;
        }
        counter += 1;
        if frames
            .write_all(&progress_frame(counter))
            .and_then(|()| frames.flush())
            .is_err()
        {
            return false;
        }
        let acquired = match acquire(ctx) {
            Ok(acquired) => acquired,
            Err(waiting) => {
                note(report, waiting);
                thread::sleep(ctx.poll);
                continue;
            }
        };
        note(report, "discovering".into());
        let dev = ctx.root.join("dev");
        let proxy = match Proxy::create(
            &dev,
            acquired.opened.input_id,
            &proxy_keys(acquired.evidence),
        ) {
            Ok(proxy) => proxy,
            Err(error) => {
                note(report, format!("proxy:{:?}", error.kind()));
                return false;
            }
        };
        let since = Instant::now();
        let ready = loop {
            match wait_ready(
                &ctx.root.join("sys"),
                &dev,
                &ctx.root.join("proc"),
                &proxy.sysname,
                Duration::ZERO,
            ) {
                Err(error)
                    if error.kind() == io::ErrorKind::TimedOut
                        && since.elapsed() < ctx.readiness
                        && !stop.load(Ordering::SeqCst) =>
                {
                    counter += 1;
                    if frames
                        .write_all(&progress_frame(counter))
                        .and_then(|()| frames.flush())
                        .is_err()
                    {
                        return false;
                    }
                    thread::sleep(READY_POLL);
                }
                result => break result,
            }
        };
        if stop.load(Ordering::SeqCst) {
            continue;
        }
        if let Err(error) = ready {
            note(report, format!("proxy_ready:{:?}", error.kind()));
            thread::sleep(ctx.poll);
            continue;
        }
        if !load(ctx.store).is_ok_and(|policy| policy.generation() == acquired.generation) {
            continue;
        }
        let mut tally = Tally::default();
        let end = forward::run(
            &acquired.opened,
            &proxy,
            acquired.evidence,
            &acquired.groups,
            ctx.forward,
            stop,
            &mut tally,
            |step| {
                counter += 1;
                let _ = frames
                    .write_all(&progress_frame(counter))
                    .and_then(|()| frames.flush());
                if step == 1 {
                    let groups: Vec<String> =
                        acquired.groups.iter().map(|g| format!("{g:?}")).collect();
                    active.store(acquired.generation, Ordering::SeqCst);
                    note(
                        report,
                        format!(
                            "active:generation={}:blocked={}",
                            acquired.generation,
                            groups.join(",")
                        ),
                    );
                }
                newer_generation.load(Ordering::SeqCst) > acquired.generation
            },
        );
        active.store(0, Ordering::SeqCst);
        drop(proxy);
        let _ = writeln!(
            report,
            "worker_session=read:{};forwarded:{};cleanup:{}",
            tally.read.summary(),
            tally.forwarded.summary(),
            tally.cleanup_releases
        );
        match end {
            Ok(End::Stopped | End::SessionLimit) => note(report, "discovering".into()),
            Ok(End::NotNeutral) => note(report, "not_neutral".into()),
            Ok(End::Fault(Fault::Disconnected | Fault::Read(_))) => {
                note(report, "disconnected".into())
            }
            Ok(End::Fault(fault)) => {
                note(report, format!("fault:{fault:?}"));
                return false;
            }
            Err(error) if error.kind() == io::ErrorKind::ResourceBusy => {
                note(report, "grab_conflict".into());
                thread::sleep(ctx.poll);
            }
            Err(error) => {
                note(report, format!("session_error:{:?}", error.kind()));
                return false;
            }
        }
    }
}
