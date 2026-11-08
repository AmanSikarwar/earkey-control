use earkey_supervisor::{
    Action, Fault, Gate, InactiveReason, Limits, LimitsError, MAX_FAILURES, Phase, Signal,
    StopCause, Supervisor,
};
use std::time::{Duration, Instant};

const START: Duration = Duration::from_millis(100);
const PROGRESS: Duration = Duration::from_millis(500);
const PAUSE: Duration = Duration::from_millis(200);
const KILL: Duration = Duration::from_millis(300);
const RESTART: Duration = Duration::from_millis(1_000);

fn supervisor() -> Supervisor {
    Supervisor::new(Limits {
        start: START,
        progress: PROGRESS,
        pause: PAUSE,
        kill: KILL,
        restart: RESTART,
    })
    .unwrap()
}

fn running(supervisor: &mut Supervisor, now: Instant) {
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Started), None);
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Progress(1)), None);
    assert_eq!(supervisor.status().phase, Phase::Running);
}

#[test]
fn zero_and_missing_deadlines_are_rejected() {
    let valid = Limits {
        start: START,
        progress: PROGRESS,
        pause: PAUSE,
        kill: KILL,
        restart: RESTART,
    };
    assert!(Supervisor::new(valid).is_ok());

    for (limits, expected) in [
        (
            Limits {
                start: Duration::ZERO,
                ..valid
            },
            LimitsError::ZeroStart,
        ),
        (
            Limits {
                progress: Duration::ZERO,
                ..valid
            },
            LimitsError::ZeroProgress,
        ),
        (
            Limits {
                pause: Duration::ZERO,
                ..valid
            },
            LimitsError::ZeroPause,
        ),
        (
            Limits {
                kill: Duration::ZERO,
                ..valid
            },
            LimitsError::ZeroKill,
        ),
        (
            Limits {
                restart: Duration::ZERO,
                ..valid
            },
            LimitsError::ZeroRestart,
        ),
    ] {
        assert_eq!(Supervisor::new(limits).err(), Some(expected));
    }
}

#[test]
fn start_timeout_stops_and_restarts() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    assert_eq!(
        supervisor.poll(start, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
    assert_eq!(
        supervisor.poll(
            start + START - Duration::from_millis(1),
            Gate::OPEN,
            Signal::None
        ),
        None
    );
    let timeout = start + START;
    assert_eq!(
        supervisor.poll(timeout, Gate::OPEN, Signal::None),
        Some(Action::Stop(StopCause::Fault(Fault::StartTimeout)))
    );
    assert_eq!(supervisor.status().failures, 1);
    assert_eq!(supervisor.poll(timeout, Gate::OPEN, Signal::Exited), None);
    assert_eq!(supervisor.status().phase, Phase::WaitingToRestart);
    assert_eq!(
        supervisor.poll(timeout + RESTART, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn stalled_progress_stops_escalates_and_restarts_without_double_counting() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);

    assert_eq!(
        supervisor.poll(
            start + PROGRESS - Duration::from_millis(1),
            Gate::OPEN,
            Signal::None
        ),
        None
    );
    let stall = start + PROGRESS;
    assert_eq!(
        supervisor.poll(stall, Gate::OPEN, Signal::None),
        Some(Action::Stop(StopCause::Fault(Fault::ProgressStalled)))
    );
    assert_eq!(supervisor.status().failures, 1);
    assert_eq!(supervisor.status().last_fault, Some(Fault::ProgressStalled));

    assert_eq!(
        supervisor.poll(stall + PAUSE, Gate::OPEN, Signal::None),
        Some(Action::Kill(StopCause::Fault(Fault::ProgressStalled)))
    );
    assert_eq!(supervisor.status().failures, 1);
    assert_eq!(
        supervisor.poll(stall + PAUSE, Gate::OPEN, Signal::Exited),
        None
    );
    assert_eq!(supervisor.status().phase, Phase::WaitingToRestart);
    assert_eq!(
        supervisor.poll(
            stall + PAUSE + RESTART - Duration::from_millis(1),
            Gate::OPEN,
            Signal::None
        ),
        None
    );
    assert_eq!(
        supervisor.poll(stall + PAUSE + RESTART, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn equal_progress_samples_do_not_refresh_the_deadline() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);

    for step in 1..5 {
        let now = start + PROGRESS * step / 5;
        assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Progress(1)), None);
    }
    assert_eq!(
        supervisor.poll(start + PROGRESS, Gate::OPEN, Signal::Progress(1)),
        Some(Action::Stop(StopCause::Fault(Fault::ProgressStalled)))
    );
}

#[test]
fn backwards_progress_is_a_fault() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);

    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(10),
            Gate::OPEN,
            Signal::Progress(0)
        ),
        Some(Action::Stop(StopCause::Fault(Fault::InvalidProgress)))
    );
    assert_eq!(supervisor.status().failures, 1);
}

#[test]
fn advancing_progress_keeps_the_worker_running() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);

    let mut now = start;
    for counter in 2..10 {
        now += PROGRESS - Duration::from_millis(1);
        assert_eq!(
            supervisor.poll(now, Gate::OPEN, Signal::Progress(counter)),
            None
        );
        assert_eq!(supervisor.status().phase, Phase::Running);
    }
    assert_eq!(
        supervisor.poll(
            now + PROGRESS - Duration::from_millis(1),
            Gate::OPEN,
            Signal::None
        ),
        None
    );
}

#[test]
fn boot_escape_stops_a_running_worker() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);
    let boot_disabled = Gate {
        boot_disabled: true,
        ..Gate::OPEN
    };

    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(10),
            boot_disabled,
            Signal::None
        ),
        Some(Action::Stop(StopCause::BootDisabled))
    );
    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(20),
            boot_disabled,
            Signal::Exited
        ),
        None
    );
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::BootDisabled)
    );
    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(20) + RESTART,
            boot_disabled,
            Signal::None
        ),
        None
    );
    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(20) + RESTART,
            Gate::OPEN,
            Signal::None
        ),
        Some(Action::Start)
    );
}

#[test]
fn ordering_pause_that_meets_its_deadline_does_not_count_a_failure() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);
    let pause = Gate {
        paused: true,
        ..Gate::OPEN
    };

    assert_eq!(
        supervisor.poll(start + Duration::from_millis(10), pause, Signal::None),
        Some(Action::Stop(StopCause::Paused))
    );
    assert_eq!(
        supervisor.poll(start + Duration::from_millis(50), pause, Signal::Exited),
        None
    );
    assert_eq!(supervisor.status().failures, 0);
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::Paused)
    );
    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(50) + RESTART,
            pause,
            Signal::None
        ),
        None
    );
    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(50) + RESTART,
            Gate::OPEN,
            Signal::None
        ),
        Some(Action::Start)
    );
}

#[test]
fn orderly_pause_that_misses_its_deadline_counts_a_failure() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);
    let pause = Gate {
        paused: true,
        ..Gate::OPEN
    };

    let requested = start + Duration::from_millis(10);
    assert_eq!(
        supervisor.poll(requested, pause, Signal::None),
        Some(Action::Stop(StopCause::Paused))
    );
    assert_eq!(supervisor.status().failures, 0);
    assert_eq!(
        supervisor.poll(requested + PAUSE, pause, Signal::None),
        Some(Action::Kill(StopCause::Paused))
    );
    assert_eq!(supervisor.status().failures, 1);
    assert_eq!(supervisor.status().last_fault, Some(Fault::StopTimedOut));
    assert_eq!(
        supervisor.poll(requested + PAUSE, pause, Signal::Exited),
        None
    );
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::Paused)
    );
    assert_eq!(
        supervisor.poll(requested + PAUSE + RESTART, pause, Signal::None),
        None
    );
    assert_eq!(
        supervisor.poll(requested + PAUSE + RESTART, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn unexpected_exit_reports_and_restarts_after_backoff() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);

    let exited = start + Duration::from_millis(10);
    assert_eq!(
        supervisor.poll(exited, Gate::OPEN, Signal::Exited),
        Some(Action::Fault(Fault::UnexpectedExit))
    );
    assert_eq!(supervisor.status().failures, 1);
    assert_eq!(supervisor.status().phase, Phase::WaitingToRestart);
    assert_eq!(
        supervisor.poll(
            exited + RESTART - Duration::from_millis(1),
            Gate::OPEN,
            Signal::None
        ),
        None
    );
    assert_eq!(
        supervisor.poll(exited + RESTART, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn start_failure_reports_and_restarts() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    assert_eq!(
        supervisor.poll(start, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(10),
            Gate::OPEN,
            Signal::StartFailed
        ),
        Some(Action::Fault(Fault::StartFailed))
    );
    assert_eq!(supervisor.status().failures, 1);
    assert_eq!(supervisor.status().phase, Phase::WaitingToRestart);
    assert_eq!(
        supervisor.poll(
            start + Duration::from_millis(10) + RESTART,
            Gate::OPEN,
            Signal::None
        ),
        Some(Action::Start)
    );
}

#[test]
fn kill_failure_is_unsafe_and_never_restarts() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);
    let pause = Gate {
        paused: true,
        ..Gate::OPEN
    };

    let requested = start + Duration::from_millis(10);
    assert_eq!(
        supervisor.poll(requested, pause, Signal::None),
        Some(Action::Stop(StopCause::Paused))
    );
    let escalated = requested + PAUSE;
    assert_eq!(
        supervisor.poll(escalated, pause, Signal::None),
        Some(Action::Kill(StopCause::Paused))
    );
    assert_eq!(
        supervisor.poll(escalated, pause, Signal::KillFailed),
        Some(Action::Unsafe(Fault::KillFailed))
    );
    assert_eq!(supervisor.status().phase, Phase::Unsafe(Fault::KillFailed));
    assert_eq!(supervisor.status().failures, 2);
    assert!(!supervisor.clear_latch());
    assert_eq!(
        supervisor.poll(escalated + RESTART, Gate::OPEN, Signal::None),
        None
    );
}

#[test]
fn unconfirmed_kill_is_unsafe_and_never_restarts() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);
    let pause = Gate {
        paused: true,
        ..Gate::OPEN
    };

    let requested = start + Duration::from_millis(10);
    assert_eq!(
        supervisor.poll(requested, pause, Signal::None),
        Some(Action::Stop(StopCause::Paused))
    );
    let escalated = requested + PAUSE;
    assert_eq!(
        supervisor.poll(escalated, pause, Signal::None),
        Some(Action::Kill(StopCause::Paused))
    );
    assert_eq!(
        supervisor.poll(escalated + KILL, pause, Signal::None),
        Some(Action::Unsafe(Fault::StopUnconfirmed))
    );
    assert_eq!(
        supervisor.status().phase,
        Phase::Unsafe(Fault::StopUnconfirmed)
    );
    assert_eq!(supervisor.status().failures, 2);
    assert!(!supervisor.clear_latch());
    assert_eq!(
        supervisor.poll(escalated + KILL + RESTART, Gate::OPEN, Signal::None),
        None
    );
}

#[test]
fn three_failures_latch_until_explicit_recovery() {
    let mut supervisor = supervisor();
    let mut now = Instant::now();
    running(&mut supervisor, now);

    now += Duration::from_millis(10);
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::Exited),
        Some(Action::Fault(Fault::UnexpectedExit))
    );
    assert_eq!(supervisor.status().failures, 1);

    now += RESTART;
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Started), None);
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Progress(2)), None);
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::Exited),
        Some(Action::Fault(Fault::UnexpectedExit))
    );
    assert_eq!(supervisor.status().failures, 2);

    now += RESTART;
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Started), None);
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Progress(3)), None);
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::Exited),
        Some(Action::PersistLatch)
    );
    assert_eq!(supervisor.status().failures, MAX_FAILURES);
    assert_eq!(supervisor.status().phase, Phase::Latched);

    assert_eq!(
        supervisor.poll(now + Duration::from_secs(30), Gate::OPEN, Signal::None),
        None
    );
    assert_eq!(
        supervisor.poll(
            now + Duration::from_secs(30),
            Gate {
                latched: true,
                ..Gate::OPEN
            },
            Signal::None
        ),
        None
    );
    assert!(supervisor.clear_latch());
    assert_eq!(supervisor.status().failures, 0);
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::Ready)
    );
    assert_eq!(
        supervisor.poll(now + Duration::from_secs(31), Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn third_failure_after_stop_confirmation_latches_without_restart() {
    let mut supervisor = supervisor();
    let mut now = Instant::now();
    running(&mut supervisor, now);

    now += Duration::from_millis(10);
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::Exited),
        Some(Action::Fault(Fault::UnexpectedExit))
    );
    now += RESTART;
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Started), None);
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Progress(2)), None);
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::Exited),
        Some(Action::Fault(Fault::UnexpectedExit))
    );
    now += RESTART;
    assert_eq!(
        supervisor.poll(now, Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Started), None);
    assert_eq!(supervisor.poll(now, Gate::OPEN, Signal::Progress(3)), None);

    let stall = now + PROGRESS;
    assert_eq!(
        supervisor.poll(stall, Gate::OPEN, Signal::None),
        Some(Action::Stop(StopCause::Fault(Fault::ProgressStalled)))
    );
    assert_eq!(supervisor.status().failures, MAX_FAILURES);
    assert_eq!(
        supervisor.poll(stall, Gate::OPEN, Signal::Exited),
        Some(Action::PersistLatch)
    );
    assert_eq!(supervisor.status().phase, Phase::Latched);
    assert_eq!(
        supervisor.poll(stall + RESTART, Gate::OPEN, Signal::None),
        None
    );
}

#[test]
fn persisted_latch_requires_explicit_clear_even_after_gate_opens() {
    let mut supervisor = supervisor();
    let now = Instant::now();
    let latched = Gate {
        latched: true,
        ..Gate::OPEN
    };

    assert_eq!(supervisor.poll(now, latched, Signal::None), None);
    assert_eq!(supervisor.status().phase, Phase::Latched);
    assert_eq!(
        supervisor.poll(now + Duration::from_secs(1), Gate::OPEN, Signal::None),
        None
    );
    assert!(supervisor.clear_latch());
    assert_eq!(
        supervisor.poll(now + Duration::from_secs(2), Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn external_latch_stops_a_running_worker_and_requires_clear() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);
    let latched = Gate {
        latched: true,
        ..Gate::OPEN
    };

    assert_eq!(
        supervisor.poll(start + Duration::from_millis(10), latched, Signal::None),
        Some(Action::Stop(StopCause::Latched))
    );
    assert_eq!(
        supervisor.poll(start + Duration::from_millis(20), latched, Signal::Exited),
        None
    );
    assert_eq!(supervisor.status().phase, Phase::Latched);
    assert_eq!(supervisor.status().failures, 0);
    assert_eq!(
        supervisor.poll(start + Duration::from_secs(10), Gate::OPEN, Signal::None),
        None
    );
    assert!(supervisor.clear_latch());
    assert_eq!(
        supervisor.poll(start + Duration::from_secs(11), Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn pause_and_boot_escape_block_start_and_resume_when_cleared() {
    let mut supervisor = supervisor();
    let now = Instant::now();

    let paused = Gate {
        paused: true,
        ..Gate::OPEN
    };
    assert_eq!(supervisor.poll(now, paused, Signal::None), None);
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::Paused)
    );

    let boot_disabled = Gate {
        boot_disabled: true,
        ..Gate::OPEN
    };
    assert_eq!(
        supervisor.poll(now + Duration::from_secs(1), boot_disabled, Signal::None),
        None
    );
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::BootDisabled)
    );

    assert_eq!(
        supervisor.poll(now + Duration::from_secs(2), Gate::OPEN, Signal::None),
        Some(Action::Start)
    );
}

#[test]
fn pause_cancels_a_pending_restart() {
    let mut supervisor = supervisor();
    let start = Instant::now();
    running(&mut supervisor, start);

    let exited = start + Duration::from_millis(10);
    assert_eq!(
        supervisor.poll(exited, Gate::OPEN, Signal::Exited),
        Some(Action::Fault(Fault::UnexpectedExit))
    );
    assert_eq!(supervisor.status().phase, Phase::WaitingToRestart);

    let paused = Gate {
        paused: true,
        ..Gate::OPEN
    };
    assert_eq!(
        supervisor.poll(exited + Duration::from_millis(20), paused, Signal::None),
        None
    );
    assert_eq!(
        supervisor.status().phase,
        Phase::Inactive(InactiveReason::Paused)
    );
    assert_eq!(
        supervisor.poll(
            exited + RESTART + Duration::from_millis(20),
            paused,
            Signal::None
        ),
        None
    );
    assert_eq!(
        supervisor.poll(
            exited + RESTART + Duration::from_millis(20),
            Gate::OPEN,
            Signal::None
        ),
        Some(Action::Start)
    );
}
