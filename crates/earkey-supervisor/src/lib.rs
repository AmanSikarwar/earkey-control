//! Host-testable supervision policy for the input worker.
//!
//! This crate is a state machine only. It does not spawn, signal or inspect a
//! process, open a device, read a clock or persist recovery state. The service
//! adapter supplies monotonic time, the durable recovery gate and worker
//! signals, then executes the returned actions.
//!
//! The worker must report progress from its actual event loop, including
//! output writes and cleanup waits. A separate heartbeat thread that keeps
//! reporting while forwarding is stuck does not satisfy this contract.
//!
//! The adapter must re-read the recovery gate immediately before executing
//! `Start`, and the worker must repeat the pre-open pause and `stay-inactive`
//! marker check before opening any candidate device. Terminating only the
//! direct child is insufficient; the adapter must stop the verified worker
//! process group and confirm exit before treating a stop as complete.
//!
//! The deadlines here are configurable inputs, not measured production
//! values. Stage 4 must select and physically validate them before any grab.
//! The supervisor never owns or inherits a grabbed descriptor; the worker and
//! its process adapter remain responsible for closing the physical node and
//! destroying the proxy during `Stop`.

use std::cmp::Ordering;
use std::time::{Duration, Instant};

/// Consecutive worker failures allowed in one supervisor session.
pub const MAX_FAILURES: u8 = 3;

/// Finite deadlines and restart spacing for one supervisor session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Maximum time from `Start` until the worker confirms startup.
    pub start: Duration,
    /// Maximum silence between progress observations while running.
    pub progress: Duration,
    /// Maximum time allowed for orderly stop and resource cleanup.
    pub pause: Duration,
    /// Maximum time to wait for a forced termination to be confirmed.
    pub kill: Duration,
    /// Delay before restarting after a worker failure.
    pub restart: Duration,
}

/// A rejected deadline configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitsError {
    ZeroStart,
    ZeroProgress,
    ZeroPause,
    ZeroKill,
    ZeroRestart,
}

impl Limits {
    /// Rejects zero deadlines. No indefinite deadline is representable.
    pub fn validate(self) -> Result<Self, LimitsError> {
        if self.start.is_zero() {
            return Err(LimitsError::ZeroStart);
        }
        if self.progress.is_zero() {
            return Err(LimitsError::ZeroProgress);
        }
        if self.pause.is_zero() {
            return Err(LimitsError::ZeroPause);
        }
        if self.kill.is_zero() {
            return Err(LimitsError::ZeroKill);
        }
        if self.restart.is_zero() {
            return Err(LimitsError::ZeroRestart);
        }
        Ok(self)
    }
}

/// Durable recovery conditions checked before starting or continuing a worker.
///
/// The service derives this from the validated policy and the pre-open
/// `stay-inactive` marker. The marker must be checked independently; policy
/// state alone cannot satisfy the boot escape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Gate {
    /// Immediate pause is active.
    pub paused: bool,
    /// The pre-open next-boot escape is set. The service must check the
    /// `stay-inactive` marker independently of policy state.
    pub boot_disabled: bool,
    /// A durable restart latch exists.
    pub latched: bool,
}

impl Gate {
    /// No pause, boot escape or persisted restart latch.
    pub const OPEN: Self = Self {
        paused: false,
        boot_disabled: false,
        latched: false,
    };

    fn inactive_reason(self) -> Option<InactiveReason> {
        // Report the boot escape first; either condition blocks a start.
        if self.boot_disabled {
            Some(InactiveReason::BootDisabled)
        } else if self.paused {
            Some(InactiveReason::Paused)
        } else {
            None
        }
    }
}

/// One observation from the service adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Signal {
    /// No new observation; use this to advance deadlines.
    None,
    /// The worker process exists and completed its startup acknowledgment.
    Started,
    /// The worker could not be started.
    StartFailed,
    /// Latest event-loop progress counter. It must increase only when the
    /// actual input, output or cleanup loop reaches a checkpoint.
    Progress(u64),
    /// The worker process exited or completed an orderly stop.
    Exited,
    /// A forced termination could not be delivered.
    KillFailed,
}

/// Why the supervisor requires a stop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopCause {
    Paused,
    BootDisabled,
    Latched,
    Fault(Fault),
}

/// Observed or inferred worker failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    /// The adapter could not create the worker.
    StartFailed,
    /// The worker did not acknowledge startup within the start deadline.
    StartTimeout,
    /// The worker exited without a stop request.
    UnexpectedExit,
    /// The progress counter moved backwards.
    InvalidProgress,
    /// The event loop made no progress within the progress deadline.
    ProgressStalled,
    /// Orderly stop missed its deadline, so a forced termination was required.
    StopTimedOut,
    /// The adapter could not deliver the forced termination.
    KillFailed,
    /// The worker did not exit within the kill deadline.
    StopUnconfirmed,
}

/// Safe inactive condition. `Latched` is represented by [`Phase::Latched`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InactiveReason {
    Ready,
    Paused,
    BootDisabled,
}

/// Current supervisor phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Inactive(InactiveReason),
    Starting,
    Running,
    Stopping {
        cause: StopCause,
        forced: bool,
    },
    WaitingToRestart,
    /// Restart is forbidden until explicit recovery clears the durable latch.
    Latched,
    /// A worker may still be running. No restart or automatic recovery is safe.
    Unsafe(Fault),
}

/// Action for the service adapter to execute, in order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    /// Create the worker process. It must not inherit grabbed descriptors.
    Start,
    /// Ask the worker to pause, release the physical node and destroy the proxy.
    Stop(StopCause),
    /// Forcefully terminate the worker after the orderly stop deadline.
    Kill(StopCause),
    /// Report a failure. A restart may already be scheduled; see [`Phase`].
    Fault(Fault),
    /// Persist the restart latch before any future start is considered. A
    /// failed write must block or terminate the service; it must not fall
    /// through to a start.
    PersistLatch,
    /// Persist the latch, report the failure and do not restart. A fresh
    /// supervisor session is required after the operator establishes that no
    /// worker survives. A failed latch write must block the service.
    Unsafe(Fault),
}

/// Current state for status reporting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Status {
    pub phase: Phase,
    pub failures: u8,
    pub last_fault: Option<Fault>,
}

/// Pure supervision state machine.
pub struct Supervisor {
    limits: Limits,
    phase: Phase,
    failures: u8,
    last_fault: Option<Fault>,
    deadline: Option<Instant>,
    started: bool,
    last_progress: u64,
    stop_counted: bool,
}

impl Supervisor {
    /// Creates an inactive supervisor. A fresh instance resets the in-memory
    /// failure count but must still honor a persisted latch supplied through
    /// [`Gate`].
    pub fn new(limits: Limits) -> Result<Self, LimitsError> {
        Ok(Self {
            limits: limits.validate()?,
            phase: Phase::Inactive(InactiveReason::Ready),
            failures: 0,
            last_fault: None,
            deadline: None,
            started: false,
            last_progress: 0,
            stop_counted: false,
        })
    }

    /// Advances the state machine. `now` must be monotonic.
    ///
    /// The adapter should call this on a bounded tick even when the worker is
    /// idle, and immediately with any new [`Signal`].
    #[must_use = "the returned action must be executed by the service adapter"]
    pub fn poll(&mut self, now: Instant, gate: Gate, signal: Signal) -> Option<Action> {
        if let Some(action) = self.handle_signal(now, signal) {
            return Some(action);
        }
        if !matches!(
            self.phase,
            Phase::Stopping { .. } | Phase::Latched | Phase::Unsafe(_)
        ) && let Some(action) = self.apply_gate(now, gate)
        {
            return Some(action);
        }
        self.check_deadline(now, gate)
    }

    /// Reports the current phase, failure count and last fault.
    pub fn status(&self) -> Status {
        Status {
            phase: self.phase,
            failures: self.failures,
            last_fault: self.last_fault,
        }
    }

    /// Explicitly clears a restart latch after the durable recovery state has
    /// been cleared and policy revalidated. Only valid when stopped and
    /// latched; an unconfirmed worker must not be recovered this way.
    pub fn clear_latch(&mut self) -> bool {
        if self.phase != Phase::Latched {
            return false;
        }
        self.phase = Phase::Inactive(InactiveReason::Ready);
        self.failures = 0;
        self.last_fault = None;
        self.deadline = None;
        self.stop_counted = false;
        true
    }

    fn handle_signal(&mut self, now: Instant, signal: Signal) -> Option<Action> {
        match signal {
            Signal::None => None,
            Signal::Started => {
                if self.phase == Phase::Starting {
                    self.started = true;
                    self.deadline = Some(after(now, self.limits.progress));
                }
                None
            }
            Signal::StartFailed => {
                if self.phase == Phase::Starting {
                    self.stopped_fault(now, Fault::StartFailed)
                } else {
                    None
                }
            }
            Signal::Progress(counter) => self.handle_progress(now, counter),
            Signal::Exited => match self.phase {
                Phase::Starting | Phase::Running => self.stopped_fault(now, Fault::UnexpectedExit),
                Phase::Stopping { .. } => self.settle(now),
                _ => None,
            },
            Signal::KillFailed => {
                if matches!(self.phase, Phase::Stopping { .. }) {
                    self.record_failure(Fault::KillFailed);
                    self.phase = Phase::Unsafe(Fault::KillFailed);
                    self.deadline = None;
                    Some(Action::Unsafe(Fault::KillFailed))
                } else {
                    None
                }
            }
        }
    }

    fn handle_progress(&mut self, now: Instant, counter: u64) -> Option<Action> {
        match self.phase {
            Phase::Starting => {
                self.started = true;
                self.last_progress = counter;
                self.phase = Phase::Running;
                self.deadline = Some(after(now, self.limits.progress));
                None
            }
            Phase::Running => match counter.cmp(&self.last_progress) {
                Ordering::Less => self.begin_fault_stop(now, Fault::InvalidProgress),
                Ordering::Equal => None,
                Ordering::Greater => {
                    self.last_progress = counter;
                    self.deadline = Some(after(now, self.limits.progress));
                    None
                }
            },
            _ => None,
        }
    }

    fn apply_gate(&mut self, now: Instant, gate: Gate) -> Option<Action> {
        if gate.latched {
            match self.phase {
                Phase::Starting | Phase::Running => {
                    return self.begin_gate_stop(now, StopCause::Latched);
                }
                Phase::Inactive(_) | Phase::WaitingToRestart => {
                    self.phase = Phase::Latched;
                    self.deadline = None;
                }
                _ => {}
            }
            return None;
        }

        match self.phase {
            Phase::Inactive(_) => {
                if let Some(reason) = gate.inactive_reason() {
                    self.phase = Phase::Inactive(reason);
                    self.deadline = None;
                    None
                } else {
                    self.start(now);
                    Some(Action::Start)
                }
            }
            Phase::Starting | Phase::Running => {
                let reason = gate.inactive_reason()?;
                let cause = match reason {
                    InactiveReason::Paused => StopCause::Paused,
                    InactiveReason::BootDisabled => StopCause::BootDisabled,
                    InactiveReason::Ready => return None,
                };
                self.begin_gate_stop(now, cause)
            }
            Phase::WaitingToRestart => {
                if let Some(reason) = gate.inactive_reason() {
                    self.phase = Phase::Inactive(reason);
                    self.deadline = None;
                }
                None
            }
            _ => None,
        }
    }

    fn check_deadline(&mut self, now: Instant, gate: Gate) -> Option<Action> {
        let deadline = self.deadline?;
        if now < deadline {
            return None;
        }
        match self.phase {
            Phase::Starting => {
                let fault = if self.started {
                    Fault::ProgressStalled
                } else {
                    Fault::StartTimeout
                };
                self.begin_fault_stop(now, fault)
            }
            Phase::Running => self.begin_fault_stop(now, Fault::ProgressStalled),
            Phase::Stopping {
                cause,
                forced: false,
            } => self.escalate(now, cause),
            Phase::Stopping { forced: true, .. } => {
                self.record_failure(Fault::StopUnconfirmed);
                self.phase = Phase::Unsafe(Fault::StopUnconfirmed);
                self.deadline = None;
                Some(Action::Unsafe(Fault::StopUnconfirmed))
            }
            Phase::WaitingToRestart => {
                if gate.paused || gate.boot_disabled || gate.latched {
                    return None;
                }
                self.start(now);
                Some(Action::Start)
            }
            _ => None,
        }
    }

    fn start(&mut self, now: Instant) {
        self.phase = Phase::Starting;
        self.started = false;
        self.last_progress = 0;
        self.stop_counted = false;
        self.deadline = Some(after(now, self.limits.start));
    }

    fn begin_fault_stop(&mut self, now: Instant, fault: Fault) -> Option<Action> {
        self.record_failure(fault);
        self.stop_counted = true;
        self.phase = Phase::Stopping {
            cause: StopCause::Fault(fault),
            forced: false,
        };
        self.deadline = Some(after(now, self.limits.pause));
        Some(Action::Stop(StopCause::Fault(fault)))
    }

    fn begin_gate_stop(&mut self, now: Instant, cause: StopCause) -> Option<Action> {
        self.stop_counted = false;
        self.phase = Phase::Stopping {
            cause,
            forced: false,
        };
        self.deadline = Some(after(now, self.limits.pause));
        Some(Action::Stop(cause))
    }

    fn escalate(&mut self, now: Instant, cause: StopCause) -> Option<Action> {
        if !self.stop_counted {
            self.record_failure(Fault::StopTimedOut);
            self.stop_counted = true;
        }
        self.phase = Phase::Stopping {
            cause,
            forced: true,
        };
        self.deadline = Some(after(now, self.limits.kill));
        Some(Action::Kill(cause))
    }

    fn settle(&mut self, now: Instant) -> Option<Action> {
        let cause = match self.phase {
            Phase::Stopping { cause, .. } => cause,
            _ => return None,
        };
        self.deadline = None;
        self.stop_counted = false;
        if self.failures >= MAX_FAILURES {
            self.phase = Phase::Latched;
            return Some(Action::PersistLatch);
        }
        if cause == StopCause::Latched {
            self.phase = Phase::Latched;
            return None;
        }
        if matches!(cause, StopCause::Fault(_)) {
            self.phase = Phase::WaitingToRestart;
            self.deadline = Some(after(now, self.limits.restart));
            return None;
        }
        self.phase = Phase::Inactive(InactiveReason::Ready);
        None
    }

    fn stopped_fault(&mut self, now: Instant, fault: Fault) -> Option<Action> {
        self.record_failure(fault);
        self.deadline = None;
        self.stop_counted = false;
        if self.failures >= MAX_FAILURES {
            self.phase = Phase::Latched;
            Some(Action::PersistLatch)
        } else {
            self.phase = Phase::WaitingToRestart;
            self.deadline = Some(after(now, self.limits.restart));
            Some(Action::Fault(fault))
        }
    }

    fn record_failure(&mut self, fault: Fault) {
        self.failures = self.failures.saturating_add(1).min(MAX_FAILURES);
        self.last_fault = Some(fault);
    }
}

/// Overflowing an `Instant` immediately times out rather than panicking.
fn after(now: Instant, limit: Duration) -> Instant {
    now.checked_add(limit).unwrap_or(now)
}
