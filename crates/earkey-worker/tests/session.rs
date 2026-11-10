#![cfg(any(target_os = "linux", target_os = "android"))]

use earkey_policy::identity::{Evidence, Group};
use earkey_supervisor::{Gate, Limits};
use earkey_worker::environment::{Observed, parse};
use earkey_worker::session::{Selection, blocked_effects, supervise, worker};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const LIMITS: Limits = Limits {
    start: Duration::from_millis(500),
    progress: Duration::from_millis(300),
    pause: Duration::from_millis(300),
    kill: Duration::from_millis(500),
    restart: Duration::from_secs(10),
};
const STARTED: &str = r"printf '\001\000\000\000\000\000\000\000\000'";
const PROGRESS: &str = r#"n=1; while [ $n -lt 250 ]; do printf "\\002\\$(printf %03o $n)\\000\\000\\000\\000\\000\\000\\000"; n=$((n+1)); sleep 0.05; done"#;

fn sh(script: &str) -> (PathBuf, Vec<String>) {
    ("/bin/sh".into(), vec!["-c".into(), script.into()])
}

fn lines(report: &earkey_worker::check::Report) -> Vec<String> {
    report
        .lines
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect()
}

#[test]
fn a_progressing_worker_runs_until_the_session_pauses_it() {
    let (exe, args) = sh(&format!("{STARTED}; {PROGRESS}"));
    // SAFETY: This test thread reaps children only through the adapter.
    let report = unsafe { supervise(exe, args, Duration::from_secs(1), LIMITS, || Ok(Gate::OPEN)) };
    let lines = lines(&report);
    assert!(report.passed, "{lines:?}");
    assert!(
        lines.contains(&"supervisor_actions=[Start, Stop(Paused)]".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"supervisor_phase=Inactive(Paused)".to_owned()),
        "{lines:?}"
    );
    let timeline = lines
        .iter()
        .find_map(|l| l.strip_prefix("supervisor_timeline="))
        .unwrap();
    let entries: Vec<&str> = timeline
        .split("; ")
        .map(|e| e.split_once(':').unwrap().1)
        .collect();
    assert_eq!(
        entries,
        [
            "Start",
            "Starting",
            "Running",
            "Stop(Paused)",
            "Stopping { cause: Paused, forced: false }",
            "Inactive(Paused)"
        ],
        "{timeline}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("supervisor_started_uptime_ms="))
    );
    let stop: u128 = lines
        .iter()
        .find_map(|l| l.strip_prefix("supervisor_stop_ms="))
        .unwrap()
        .parse()
        .unwrap();
    assert!(stop < 300, "{stop}");
}

#[test]
fn a_worker_that_never_starts_is_a_fault_not_a_pass() {
    let (exe, args) = sh("sleep 30");
    // SAFETY: As above.
    let report = unsafe { supervise(exe, args, Duration::from_secs(1), LIMITS, || Ok(Gate::OPEN)) };
    let lines = lines(&report);
    assert!(!report.passed);
    assert!(
        lines.contains(&"supervisor_last_fault=StartTimeout".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"supervisor_running_after_ms=never".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"supervisor_failures=1".to_owned()),
        "{lines:?}"
    );
    let timeline = lines
        .iter()
        .find_map(|l| l.strip_prefix("supervisor_timeline="))
        .unwrap();
    assert!(
        timeline.contains(":Stop(Fault(StartTimeout))"),
        "{timeline}"
    );
    assert!(timeline.contains(":WaitingToRestart"), "{timeline}");
}

#[test]
fn an_unreadable_gate_never_starts_the_worker() {
    let (exe, args) = sh(&format!("{STARTED}; {PROGRESS}"));
    // SAFETY: As above.
    let report = unsafe {
        supervise(exe, args, Duration::from_secs(1), LIMITS, || {
            Err(io::ErrorKind::PermissionDenied.into())
        })
    };
    let lines = lines(&report);
    assert!(!report.passed);
    assert!(
        lines.contains(&"supervisor_actions=[]".to_owned()),
        "{lines:?}"
    );
    assert!(
        lines.contains(&"supervisor_running_after_ms=never".to_owned()),
        "{lines:?}"
    );
}

fn environment() -> Observed {
    parse(
        |name| {
            Some(
                match name {
                    "ro.product.model" => "M2101K6P",
                    "ro.product.device" => "sweetin",
                    "ro.build.version.sdk" => "37",
                    "ro.custom.version" => "PixelOS_sweet-17.0-20261030-2116",
                    "ro.build.id" => "CP2A.260605.016",
                    "ro.build.version.incremental" => "1793394994",
                    _ => return None,
                }
                .to_owned(),
            )
        },
        "Linux version 4.14.357-reforge-g9b21e3df6c0a (b@h) (c) #2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026",
    )
    .unwrap()
}

#[test]
fn the_worker_checks_the_gate_and_acquisition_before_anything_else() {
    let stop = AtomicBool::new(false);
    let root = Path::new("/nonexistent-earkey-root");
    let paused = Gate {
        paused: true,
        ..Gate::OPEN
    };
    for (gate, expected) in [
        (Ok(paused), "worker_gate=closed\n"),
        (
            Err(io::ErrorKind::PermissionDenied.into()),
            "worker_gate=error:PermissionDenied\n",
        ),
    ] {
        let (mut frames, mut report) = (Vec::new(), Vec::new());
        assert!(!worker(
            root,
            &environment(),
            &Selection::Passthrough,
            || gate,
            &stop,
            &mut frames,
            &mut report
        ));
        assert!(frames.is_empty());
        assert_eq!(String::from_utf8(report).unwrap(), expected);
    }
    let (mut frames, mut report) = (Vec::new(), Vec::new());
    assert!(!worker(
        root,
        &environment(),
        &Selection::Passthrough,
        || Ok(Gate::OPEN),
        &stop,
        &mut frames,
        &mut report
    ));
    assert!(frames.is_empty());
    assert_eq!(
        String::from_utf8(report).unwrap(),
        "worker_layout=unavailable:NotFound\nworker_check=fail\n"
    );
}

#[test]
fn selections_parse_strictly() {
    assert_eq!(
        Selection::parse("passthrough"),
        Some(Selection::Passthrough)
    );
    assert_eq!(Selection::parse("block-all"), Some(Selection::BlockAll));
    assert_eq!(
        Selection::parse("volume-up,center"),
        Some(Selection::Groups(vec![Group::VolumeUp, Group::Center]))
    );
    for bad in [
        "",
        "center,",
        "center,center",
        "Center",
        "volume_up",
        "all",
        "center volume-up",
    ] {
        assert_eq!(Selection::parse(bad), None, "{bad:?}");
    }
    let evidence = Evidence::XiaomiPixelosReforge20261110;
    assert_eq!(
        Selection::BlockAll.groups(evidence),
        [Group::Center, Group::VolumeUp, Group::VolumeDown]
    );
    assert!(Selection::Passthrough.groups(evidence).is_empty());
}

#[test]
fn blocking_center_discloses_its_shared_effects() {
    let evidence = Evidence::XiaomiPixelosReforge20261110;
    let center = blocked_effects(evidence, &[Group::Center]);
    for effect in [
        "Play or pause",
        "Open assistant and wake screen",
        "Answer or end cellular call",
        "Reject an incoming call",
    ] {
        assert!(center.contains(effect), "{center}");
    }
    assert!(!center.contains("volume"), "{center}");
    assert_eq!(blocked_effects(evidence, &[]), "none");
    assert!(blocked_effects(evidence, &[Group::VolumeUp]).contains("Raise volume"));
}

mod daemon {
    use super::*;
    use earkey_supervisor::Phase;
    use earkey_worker::session::daemon;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    const FAST: Limits = Limits {
        start: Duration::from_millis(500),
        progress: Duration::from_millis(300),
        pause: Duration::from_millis(300),
        kill: Duration::from_millis(500),
        restart: Duration::from_millis(50),
    };

    fn wait(what: &str, done: impl Fn() -> bool) {
        let end = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < end, "timed out waiting for {what}");
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn a_running_worker_is_supervised_until_stop() {
        let (exe, args) = sh(&format!("{STARTED}; {PROGRESS}"));
        let stop = AtomicBool::new(false);
        let statuses = Mutex::new(Vec::<String>::new());
        let phase = thread::scope(|scope| {
            let daemon = scope.spawn(|| {
                // SAFETY: This test thread reaps children only through the adapter.
                unsafe {
                    daemon(
                        exe,
                        args,
                        FAST,
                        || Ok(Gate::OPEN),
                        || Ok(()),
                        &stop,
                        |s| statuses.lock().unwrap().push(s.to_owned()),
                    )
                }
            });
            wait("running", || {
                statuses
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|s| s.starts_with("phase=Running"))
            });
            thread::sleep(Duration::from_millis(500));
            stop.store(true, Ordering::SeqCst);
            daemon.join().unwrap()
        });
        assert_eq!(
            phase.unwrap(),
            Phase::Inactive(earkey_supervisor::InactiveReason::Paused)
        );
        let statuses = statuses.into_inner().unwrap();
        assert!(
            statuses.iter().all(|s| s.contains("failures=0")),
            "{statuses:?}"
        );
    }

    #[test]
    fn repeated_failures_persist_the_latch_and_clearing_it_restarts() {
        let (exe, args) = sh("exit 1");
        let stop = AtomicBool::new(false);
        let latched = Arc::new(AtomicBool::new(false));
        let writes = AtomicU32::new(0);
        let statuses = Mutex::new(Vec::<String>::new());
        let phase = thread::scope(|scope| {
            let daemon = scope.spawn(|| {
                // SAFETY: As above.
                unsafe {
                    daemon(
                        exe,
                        args,
                        FAST,
                        || {
                            Ok(Gate {
                                latched: latched.load(Ordering::SeqCst),
                                ..Gate::OPEN
                            })
                        },
                        || {
                            writes.fetch_add(1, Ordering::SeqCst);
                            latched.store(true, Ordering::SeqCst);
                            Ok(())
                        },
                        &stop,
                        |s| statuses.lock().unwrap().push(s.to_owned()),
                    )
                }
            });
            wait("first latch", || writes.load(Ordering::SeqCst) == 1);
            thread::sleep(Duration::from_millis(300));
            assert_eq!(writes.load(Ordering::SeqCst), 1, "no restart while latched");
            latched.store(false, Ordering::SeqCst);
            wait("second latch after clearing", || {
                writes.load(Ordering::SeqCst) == 2
            });
            stop.store(true, Ordering::SeqCst);
            daemon.join().unwrap()
        });
        assert_eq!(phase.unwrap(), Phase::Latched);
        let statuses = statuses.into_inner().unwrap();
        assert!(
            statuses
                .iter()
                .any(|s| s.starts_with("phase=Latched\nfailures=3")),
            "{statuses:?}"
        );
    }

    #[test]
    fn a_failed_latch_write_stops_the_daemon() {
        let (exe, args) = sh("exit 1");
        let stop = AtomicBool::new(false);
        // SAFETY: As above.
        let result = unsafe {
            daemon(
                exe,
                args,
                FAST,
                || Ok(Gate::OPEN),
                || Err(io::ErrorKind::PermissionDenied.into()),
                &stop,
                |_| {},
            )
        };
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }
}
