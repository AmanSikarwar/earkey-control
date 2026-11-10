use earkey_policy::store::Store;
use earkey_supervisor::process::{Adapter, Host, Lock, progress_frame, started_frame};
use earkey_supervisor::{
    Action, Fault, Gate, InactiveReason, Limits, Phase, StopCause, Supervisor, store,
};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant};

const LOCK: &str = "/data/adb/earkey_control/supervisor.lock";
const TICK: Duration = Duration::from_millis(10);
const PHASE_DEADLINE: Duration = Duration::from_secs(5);
const WORKER_LIFETIME: Duration = Duration::from_secs(30);
const WORKER_PERIOD: Duration = Duration::from_millis(20);
const LIMITS: Limits = Limits {
    start: Duration::from_secs(2),
    progress: Duration::from_millis(500),
    pause: Duration::from_secs(1),
    kill: Duration::from_secs(1),
    restart: Duration::from_millis(200),
};

pub(crate) fn run(command: &str) -> ExitCode {
    match command {
        "drill" => drill(),
        "worker" => worker(false, false),
        "worker-ignore-term" => worker(true, false),
        _ => worker(false, true),
    }
}

fn worker(ignore_term: bool, stall: bool) -> ExitCode {
    if ignore_term {
        // SAFETY: Installing SIG_IGN has no handler or memory preconditions.
        unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
    }
    let mut out = io::stdout().lock();
    let mut write = |frame: &[u8]| out.write_all(frame).and_then(|()| out.flush());
    if write(&started_frame()).is_err() {
        return ExitCode::FAILURE;
    }
    let end = Instant::now() + WORKER_LIFETIME;
    let mut counter = 0;
    while Instant::now() < end {
        if !stall || counter == 0 {
            counter += 1;
            if write(&progress_frame(counter)).is_err() {
                return ExitCode::FAILURE;
            }
        }
        thread::sleep(WORKER_PERIOD);
    }
    ExitCode::SUCCESS
}

struct DrillHost {
    store: Store,
    exe: PathBuf,
    mode: &'static str,
    paused: bool,
}

impl DrillHost {
    fn open(mode: &'static str) -> io::Result<Self> {
        Ok(Self {
            store: Store::open().map_err(|error| io::Error::other(format!("{error:?}")))?,
            exe: std::env::current_exe()?,
            mode,
            paused: false,
        })
    }
}

impl Host for DrillHost {
    fn gate(&mut self) -> io::Result<Gate> {
        let mut gate = store::read_gate(&self.store)?;
        gate.paused |= self.paused;
        Ok(gate)
    }

    fn command(&mut self) -> Command {
        let mut command = Command::new(&self.exe);
        command.arg(self.mode);
        command
    }

    fn persist_latch(&mut self) -> io::Result<()> {
        Err(io::Error::other("drill_does_not_persist"))
    }
}

struct Session {
    adapter: Adapter,
    supervisor: Supervisor,
    host: DrillHost,
}

impl Session {
    fn open(mode: &'static str) -> io::Result<Self> {
        Ok(Self {
            // SAFETY: The drill reaps only through adapters, each used on its own thread.
            adapter: unsafe { Adapter::new() },
            supervisor: Supervisor::new(LIMITS).expect("drill limits are non-zero"),
            host: DrillHost::open(mode)?,
        })
    }

    fn until(&mut self, phase: Phase) -> io::Result<Vec<Action>> {
        let end = Instant::now() + PHASE_DEADLINE;
        let mut actions = Vec::new();
        while self.supervisor.status().phase != phase {
            if Instant::now() >= end {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("{:?}", self.supervisor.status()),
                ));
            }
            if let Some(action) = self
                .adapter
                .step(&mut self.supervisor, &mut self.host, TICK)?
            {
                actions.push(action);
            }
        }
        Ok(actions)
    }
}

fn paused_stop(name: &str, mode: &'static str, expected: &[Action]) -> io::Result<bool> {
    let mut session = Session::open(mode)?;
    session.until(Phase::Running)?;
    let pid = session.adapter.worker_pid().unwrap_or(0);
    session.host.paused = true;
    let start = Instant::now();
    let actions = session.until(Phase::Inactive(InactiveReason::Paused))?;
    let elapsed = start.elapsed().as_millis();
    let gone = !alive(pid);
    println!("{name}_ms={elapsed}");
    println!("{name}_actions={actions:?}");
    println!("{name}_worker_gone={gone}");
    Ok(actions == expected && gone)
}

fn stall() -> io::Result<bool> {
    let mut session = Session::open("worker-stall")?;
    session.until(Phase::Running)?;
    let start = Instant::now();
    let actions = session.until(Phase::WaitingToRestart)?;
    println!("stall_ms={}", start.elapsed().as_millis());
    println!("stall_actions={actions:?}");
    Ok(actions.first() == Some(&Action::Stop(StopCause::Fault(Fault::ProgressStalled))))
}

fn parent_death() -> io::Result<bool> {
    let pid = thread::spawn(|| -> io::Result<u32> {
        let mut session = Session::open("worker-ignore-term")?;
        session.until(Phase::Running)?;
        let pid = session.adapter.worker_pid().unwrap_or(0);
        std::mem::forget(session.adapter);
        Ok(pid)
    })
    .join()
    .map_err(|_| io::Error::other("supervising thread panicked"))??;
    let start = Instant::now();
    while alive(pid) && start.elapsed() < PHASE_DEADLINE {
        thread::sleep(TICK);
    }
    let gone = !alive(pid);
    println!("parent_death_ms={}", start.elapsed().as_millis());
    println!("parent_death_worker_gone={gone}");
    Ok(gone)
}

fn alive(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
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

fn report(name: &str, result: io::Result<bool>) -> bool {
    match result {
        Ok(passed) => {
            println!("{name}={}", if passed { "pass" } else { "fail" });
            passed
        }
        Err(error) => {
            println!("{name}=error");
            println!("{name}_error={:?}", error.kind());
            false
        }
    }
}

fn drill() -> ExitCode {
    let store = match Store::open() {
        Ok(store) => store,
        Err(error) => {
            println!("drill=storage_unavailable");
            println!("storage_error={error:?}");
            return ExitCode::FAILURE;
        }
    };
    match store::read_gate(&store) {
        Ok(gate) if gate == Gate::OPEN => println!("gate=open"),
        Ok(gate) => {
            println!("drill=gate_closed");
            println!("gate={gate:?}");
            return ExitCode::FAILURE;
        }
        Err(error) => {
            println!("drill=gate_unavailable");
            println!("gate_error={:?}", error.kind());
            return ExitCode::FAILURE;
        }
    }
    let lock = match Lock::acquire(Path::new(LOCK)) {
        Ok(lock) => lock,
        Err(error) => {
            println!("drill=lock_unavailable");
            println!("lock_error={:?}", error.kind());
            return ExitCode::FAILURE;
        }
    };
    let exclusive = Lock::acquire(Path::new(LOCK))
        .err()
        .is_some_and(|error| error.kind() == io::ErrorKind::WouldBlock);
    println!("lock_exclusive={exclusive}");

    let pause = report(
        "pause",
        paused_stop("pause", "worker", &[Action::Stop(StopCause::Paused)]),
    );
    let kill = report(
        "kill",
        paused_stop(
            "kill",
            "worker-ignore-term",
            &[
                Action::Stop(StopCause::Paused),
                Action::Kill(StopCause::Paused),
            ],
        ),
    );
    let stall = report("stall", stall());
    let parent_death = report("parent_death", parent_death());

    drop(lock);
    let removed = std::fs::remove_file(LOCK).is_ok();
    println!("lock_removed={removed}");
    if exclusive && pause && kill && stall && parent_death && removed {
        println!("drill=pass");
        ExitCode::SUCCESS
    } else {
        println!("drill=fail");
        ExitCode::FAILURE
    }
}
