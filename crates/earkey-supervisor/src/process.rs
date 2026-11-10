//! Linux process adapter for [`Supervisor`].
//!
//! The worker runs in its own process group with a parent-death signal and
//! reports fixed-size frames on stdout. Stop sends `SIGTERM` and kill sends
//! `SIGKILL` to the whole group. Exit is confirmed only after the unreaped
//! leader has exited and no live process remains in its group; the zombie
//! leader keeps the group ID reserved until then. The worker must not leave
//! its process group.

use crate::{Action, Gate, Signal, Supervisor};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Size of every worker frame: a tag byte and a little-endian counter.
pub const FRAME: usize = 9;
const STARTED: u8 = 1;
const PROGRESS: u8 = 2;

/// Worker-side startup acknowledgment.
pub fn started_frame() -> [u8; FRAME] {
    let mut frame = [0; FRAME];
    frame[0] = STARTED;
    frame
}

/// Worker-side event-loop progress checkpoint.
pub fn progress_frame(counter: u64) -> [u8; FRAME] {
    let mut frame = [0; FRAME];
    frame[0] = PROGRESS;
    frame[1..].copy_from_slice(&counter.to_le_bytes());
    frame
}

/// Exclusive single-instance lock, held until dropped.
pub struct Lock {
    _file: File,
}

impl Lock {
    /// Fails with `WouldBlock` while another instance holds the lock. The
    /// caller must supply a path inside validated private storage.
    pub fn acquire(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?;
        // SAFETY: The descriptor is valid for the call.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { _file: file })
    }
}

/// Service inputs and the durable latch write.
pub trait Host {
    /// Current recovery gate. An error keeps the worker stopped.
    fn gate(&mut self) -> io::Result<Gate>;
    /// A fresh worker command. Its stdio is replaced by the adapter.
    fn command(&mut self) -> Command;
    /// Persists the restart latch. An error is returned from [`Adapter::step`].
    fn persist_latch(&mut self) -> io::Result<()>;
}

const CLOSED: Gate = Gate {
    paused: true,
    boot_disabled: true,
    latched: false,
};

/// Executes supervisor actions against at most one worker process group.
pub struct Adapter {
    worker: Option<Worker>,
    pending: Signal,
}

impl Adapter {
    /// # Safety
    /// No other code may reap this process's children, and `SIGCHLD` must not
    /// be ignored. The adapter must live on the thread that owns supervision:
    /// the worker's parent-death signal fires when that thread exits.
    pub unsafe fn new() -> Self {
        Self {
            worker: None,
            pending: Signal::None,
        }
    }

    /// Observes one signal, advances the supervisor and executes its action.
    /// Waits at most `tick` for worker frames.
    pub fn step(
        &mut self,
        supervisor: &mut Supervisor,
        host: &mut impl Host,
        tick: Duration,
    ) -> io::Result<Option<Action>> {
        let gate = host.gate().unwrap_or(CLOSED);
        let signal = self.observe(tick);
        let action = supervisor.poll(Instant::now(), gate, signal);
        if let Some(action) = action {
            self.execute(action, host)?;
        }
        Ok(action)
    }

    /// Process ID of the current worker group leader.
    pub fn worker_pid(&self) -> Option<u32> {
        self.worker.as_ref().map(|worker| worker.child.id())
    }

    fn observe(&mut self, tick: Duration) -> Signal {
        let pending = std::mem::replace(&mut self.pending, Signal::None);
        if pending != Signal::None {
            return pending;
        }
        let Some(worker) = &mut self.worker else {
            thread::sleep(tick);
            return Signal::None;
        };
        if let Ok(true) = worker.exited() {
            self.worker = None;
            return Signal::Exited;
        }
        worker.next(tick)
    }

    fn execute(&mut self, action: Action, host: &mut impl Host) -> io::Result<()> {
        match action {
            Action::Start => {
                if host.gate().is_ok_and(|gate| gate == Gate::OPEN) {
                    match Worker::spawn(&mut host.command()) {
                        Ok(worker) => self.worker = Some(worker),
                        Err(_) => self.pending = Signal::StartFailed,
                    }
                }
            }
            Action::Stop(_) => match &self.worker {
                Some(worker) => {
                    let _ = signal_group(worker.child.id(), libc::SIGTERM);
                }
                None => self.pending = Signal::Exited,
            },
            Action::Kill(_) => match &self.worker {
                Some(worker) => {
                    if signal_group(worker.child.id(), libc::SIGKILL).is_err() {
                        self.pending = Signal::KillFailed;
                    }
                }
                None => self.pending = Signal::Exited,
            },
            Action::Fault(_) => {}
            Action::PersistLatch | Action::Unsafe(_) => host.persist_latch()?,
        }
        Ok(())
    }
}

impl Drop for Adapter {
    fn drop(&mut self) {
        if let Some(worker) = &mut self.worker {
            let _ = signal_group(worker.child.id(), libc::SIGKILL);
            let _ = worker.child.try_wait();
        }
    }
}

struct Worker {
    child: Child,
    progress: Option<ChildStdout>,
    buffer: Vec<u8>,
}

impl Worker {
    fn spawn(command: &mut Command) -> io::Result<Self> {
        // SAFETY: getpid has no preconditions.
        let parent = unsafe { libc::getpid() };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0);
        // SAFETY: prctl and getppid are async-signal-safe and touch no memory.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    return Err(io::Error::from_raw_os_error(libc::ESRCH));
                }
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        let progress = child.stdout.take();
        Ok(Self {
            child,
            progress,
            buffer: Vec::with_capacity(FRAME * 16),
        })
    }

    fn next(&mut self, tick: Duration) -> Signal {
        if self.buffer.len() < FRAME {
            self.fill(tick);
        }
        if self.buffer.len() < FRAME {
            return Signal::None;
        }
        let frame: Vec<u8> = self.buffer.drain(..FRAME).collect();
        let counter = u64::from_le_bytes(frame[1..].try_into().unwrap());
        match (frame[0], counter) {
            (STARTED, 0) => Signal::Started,
            (PROGRESS, counter) => Signal::Progress(counter),
            // ponytail: a corrupt channel stops reporting; the progress deadline stops the worker.
            _ => {
                self.progress = None;
                self.buffer.clear();
                Signal::None
            }
        }
    }

    fn fill(&mut self, tick: Duration) {
        let Some(pipe) = &mut self.progress else {
            thread::sleep(tick);
            return;
        };
        let mut poll = libc::pollfd {
            fd: pipe.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout = tick.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: One valid pollfd is passed for the duration of the call.
        if unsafe { libc::poll(&mut poll, 1, timeout) } <= 0 {
            return;
        }
        let mut chunk = [0; FRAME * 16];
        match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => self.progress = None,
            Ok(read) => self.buffer.extend_from_slice(&chunk[..read]),
        }
        if self.buffer.len() > FRAME * 64 {
            self.progress = None;
            self.buffer.clear();
        }
    }

    fn exited(&mut self) -> io::Result<bool> {
        let pid = self.child.id();
        // SAFETY: An all-zero siginfo_t is valid output storage.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: The pointer is valid; WNOWAIT leaves the leader unreaped so
        // its PID and process group ID cannot be reused.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                pid,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: waitid filled the structure.
        if unsafe { info.si_pid() } == 0 {
            return Ok(false);
        }
        signal_group(pid, libc::SIGKILL)?;
        if live_members(pid)? {
            return Ok(false);
        }
        self.child.wait()?;
        Ok(true)
    }
}

fn signal_group(leader: u32, signal: libc::c_int) -> io::Result<()> {
    // SAFETY: kill takes no pointers. The unreaped leader reserves the group ID.
    if unsafe { libc::kill(-(leader as libc::pid_t), signal) } == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(error)
    }
}

/// True while any readable non-zombie process other than the leader is in the group.
fn live_members(leader: u32) -> io::Result<bool> {
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == leader {
            continue;
        }
        let stat = match fs::read_to_string(entry.path().join("stat")) {
            Ok(stat) => stat,
            // ponytail: a worker in the supervisor's own SELinux domain stays readable; denied entries belong to other domains.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                ) =>
            {
                continue;
            }
            Err(error) => return Err(error),
        };
        let mut fields = stat
            .rsplit_once(')')
            .ok_or(io::ErrorKind::InvalidData)?
            .1
            .split_whitespace();
        let state = fields.next().ok_or(io::ErrorKind::InvalidData)?;
        let group = fields.nth(1).ok_or(io::ErrorKind::InvalidData)?;
        if group == leader.to_string() && !matches!(state, "Z" | "X") {
            return Ok(true);
        }
    }
    Ok(false)
}
