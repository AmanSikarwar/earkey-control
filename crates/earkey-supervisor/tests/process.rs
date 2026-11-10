#![cfg(any(target_os = "linux", target_os = "android"))]

use earkey_supervisor::process::{Adapter, Host, Lock};
use earkey_supervisor::{
    Action, Fault, Gate, InactiveReason, Limits, Phase, StopCause, Supervisor,
};
use std::cell::Cell;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(20);
const STARTED: &str = r"printf '\001\000\000\000\000\000\000\000\000'";
const PROGRESS: &str = r"printf '\002\001\000\000\000\000\000\000\000'";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "earkey-process-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Fake {
    script: String,
    gates: Vec<io::Result<Gate>>,
    gate: Gate,
    gate_fails: bool,
    latches: Cell<u32>,
}

impl Fake {
    fn new(script: &str) -> Self {
        Self {
            script: script.into(),
            gates: Vec::new(),
            gate: Gate::OPEN,
            gate_fails: false,
            latches: Cell::new(0),
        }
    }
}

impl Host for Fake {
    fn gate(&mut self) -> io::Result<Gate> {
        if !self.gates.is_empty() {
            return self.gates.remove(0);
        }
        if self.gate_fails {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(self.gate)
    }

    fn command(&mut self) -> Command {
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg(&self.script);
        command
    }

    fn persist_latch(&mut self) -> io::Result<()> {
        self.latches.set(self.latches.get() + 1);
        Ok(())
    }
}

fn supervisor() -> Supervisor {
    Supervisor::new(Limits {
        start: Duration::from_secs(2),
        progress: Duration::from_millis(300),
        pause: Duration::from_millis(300),
        kill: Duration::from_secs(1),
        restart: Duration::from_millis(50),
    })
    .unwrap()
}

fn run_until(
    adapter: &mut Adapter,
    supervisor: &mut Supervisor,
    host: &mut Fake,
    done: impl Fn(&Supervisor) -> bool,
) -> Vec<Action> {
    let end = Instant::now() + Duration::from_secs(5);
    let mut actions = Vec::new();
    while !done(supervisor) {
        assert!(
            Instant::now() < end,
            "timed out in {:?}",
            supervisor.status()
        );
        if let Some(action) = adapter.step(supervisor, host, TICK).unwrap() {
            actions.push(action);
        }
    }
    actions
}

fn phase(phase: Phase) -> impl Fn(&Supervisor) -> bool {
    move |supervisor| supervisor.status().phase == phase
}

fn alive(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            let state = stat
                .rsplit_once(')')?
                .1
                .split_whitespace()
                .next()?
                .to_owned();
            Some(!matches!(state.as_str(), "Z" | "X"))
        })
        .unwrap_or(false)
}

fn wait_dead(pid: u32) -> bool {
    let end = Instant::now() + Duration::from_secs(2);
    while alive(pid) {
        if Instant::now() >= end {
            return false;
        }
        std::thread::sleep(TICK);
    }
    true
}

#[test]
fn running_worker_stops_on_pause_and_its_group_is_confirmed_gone() {
    let mut host = Fake::new(&format!(
        "{STARTED}; {PROGRESS}; trap 'exit 0' TERM; while :; do sleep 0.05; done"
    ));
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::Running),
    );
    let pid = adapter.worker_pid().unwrap();

    host.gate.paused = true;
    let actions = run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::Inactive(InactiveReason::Paused)),
    );
    assert_eq!(actions, [Action::Stop(StopCause::Paused)]);
    assert_eq!(adapter.worker_pid(), None);
    assert!(!alive(pid));
    assert_eq!(supervisor.status().failures, 0);
}

#[test]
fn worker_ignoring_terminate_is_killed() {
    let mut host = Fake::new(&format!(
        "trap '' TERM; {STARTED}; {PROGRESS}; while :; do sleep 0.05; done"
    ));
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::Running),
    );
    let pid = adapter.worker_pid().unwrap();

    host.gate.paused = true;
    let actions = run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::Inactive(InactiveReason::Paused)),
    );
    assert_eq!(
        actions,
        [
            Action::Stop(StopCause::Paused),
            Action::Kill(StopCause::Paused)
        ]
    );
    assert!(!alive(pid));
    assert_eq!(supervisor.status().last_fault, Some(Fault::StopTimedOut));
}

#[test]
fn stalled_worker_is_stopped_and_restart_is_scheduled() {
    let mut host = Fake::new(&format!("{STARTED}; {PROGRESS}; sleep 30"));
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    let actions = run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::WaitingToRestart),
    );
    assert!(actions.contains(&Action::Stop(StopCause::Fault(Fault::ProgressStalled))));
    assert_eq!(adapter.worker_pid(), None);
}

#[test]
fn exit_is_unconfirmed_until_an_escaped_descendant_is_gone() {
    let scratch = Scratch::new();
    let child = scratch.0.join("child");
    let mut host = Fake::new(&format!(
        "{STARTED}; {PROGRESS}; trap '' TERM; sleep 30 & echo $! > {}; exit 0",
        child.display()
    ));
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    let actions = run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::WaitingToRestart),
    );
    assert_eq!(actions.last(), Some(&Action::Fault(Fault::UnexpectedExit)));
    let pid: u32 = fs::read_to_string(child).unwrap().trim().parse().unwrap();
    assert!(!alive(pid));
}

#[test]
fn malformed_frame_ends_reporting_until_progress_stalls() {
    let mut host = Fake::new(&format!(
        r"{STARTED}; printf '\377\000\000\000\000\000\000\000\000'; {PROGRESS}; sleep 30"
    ));
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    let actions = run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::WaitingToRestart),
    );
    assert!(actions.contains(&Action::Stop(StopCause::Fault(Fault::ProgressStalled))));
}

#[test]
fn unreadable_gate_prevents_start() {
    let mut host = Fake::new("exit 0");
    host.gate_fails = true;
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    for _ in 0..5 {
        assert_eq!(
            adapter.step(&mut supervisor, &mut host, TICK).unwrap(),
            None
        );
    }
    assert_eq!(adapter.worker_pid(), None);
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::BootDisabled)
    );
}

#[test]
fn start_rechecks_the_gate_and_spawns_nothing_when_it_closed() {
    let scratch = Scratch::new();
    let marker = scratch.0.join("spawned");
    let mut host = Fake::new(&format!("touch {}; exit 0", marker.display()));
    let paused = Gate {
        paused: true,
        ..Gate::OPEN
    };
    host.gates = vec![Ok(Gate::OPEN), Ok(paused)];
    host.gate = paused;
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    let actions = run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::Inactive(InactiveReason::Paused)),
    );
    assert_eq!(actions, [Action::Start, Action::Stop(StopCause::Paused)]);
    assert!(!marker.exists());
    assert_eq!(supervisor.status().failures, 0);
}

#[test]
fn three_failures_persist_the_latch_once() {
    let mut host = Fake::new("exit 1");
    let mut supervisor = supervisor();
    let mut adapter = unsafe { Adapter::new() };
    let actions = run_until(
        &mut adapter,
        &mut supervisor,
        &mut host,
        phase(Phase::Latched),
    );
    assert_eq!(actions.last(), Some(&Action::PersistLatch));
    assert_eq!(host.latches.get(), 1);
    for _ in 0..5 {
        assert_eq!(
            adapter.step(&mut supervisor, &mut host, TICK).unwrap(),
            None
        );
    }
    assert_eq!(adapter.worker_pid(), None);
}

#[test]
fn worker_dies_with_the_supervising_thread() {
    let pid = std::thread::spawn(|| {
        let mut host = Fake::new(&format!(
            "trap '' TERM; {STARTED}; {PROGRESS}; while :; do sleep 0.05; done"
        ));
        let mut supervisor = supervisor();
        let mut adapter = unsafe { Adapter::new() };
        run_until(
            &mut adapter,
            &mut supervisor,
            &mut host,
            phase(Phase::Running),
        );
        let pid = adapter.worker_pid().unwrap();
        std::mem::forget(adapter);
        pid
    })
    .join()
    .unwrap();
    assert!(wait_dead(pid));
}

#[test]
fn lock_is_exclusive_until_dropped() {
    let scratch = Scratch::new();
    let path = scratch.0.join("supervisor.lock");
    let lock = Lock::acquire(&path).unwrap();
    assert_eq!(
        Lock::acquire(&path).err().map(|error| error.kind()),
        Some(io::ErrorKind::WouldBlock)
    );
    drop(lock);
    Lock::acquire(&path).unwrap();
}
