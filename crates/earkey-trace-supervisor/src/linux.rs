use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_DEADLINE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(10);
const REAP_DEADLINE: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum Outcome {
    Exited(ExitStatus),
    TimedOut,
}

struct OwnedGroup {
    child: Option<Child>,
}

impl OwnedGroup {
    fn signal_group(&self) -> io::Result<()> {
        // The unreaped group leader reserves this PID until cleanup completes.
        let result = unsafe {
            libc::kill(
                -(self.child.as_ref().unwrap().id() as libc::pid_t),
                libc::SIGKILL,
            )
        };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(())
        } else {
            Err(error)
        }
    }

    fn finish(&mut self) -> io::Result<ExitStatus> {
        self.signal_group()?;
        let end = Instant::now() + REAP_DEADLINE;
        loop {
            match self.child.as_mut().unwrap().try_wait() {
                Ok(Some(status)) => {
                    self.child.take();
                    return Ok(status);
                }
                Ok(None) => {}
                Err(error) => {
                    if error.raw_os_error() == Some(libc::ECHILD) {
                        self.child.take();
                    }
                    return Err(error);
                }
            }
            if Instant::now() >= end {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "child cleanup not confirmed",
                ));
            }
            thread::sleep(POLL.min(end.saturating_duration_since(Instant::now())));
        }
    }
}

impl Drop for OwnedGroup {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.signal_group();
            let mut child = self.child.take().unwrap();
            if !matches!(child.try_wait(), Ok(Some(_))) {
                let _ = thread::Builder::new()
                    .name("earkey-trace-reaper".into())
                    .spawn(move || {
                        let _ = child.wait();
                    });
            }
        }
    }
}

/// Runs an internally selected child in a new process group and signals that
/// group before reaping its leader. The caller must validate the executable.
/// A successful return does not confirm all descendants have exited.
///
/// # Safety
/// No other thread, signal handler, or library may reap this child, and SIGCHLD
/// must not be configured for automatic reaping while this function runs.
pub unsafe fn run(command: &mut Command, deadline: Duration) -> io::Result<Outcome> {
    if deadline.is_zero() || deadline > MAX_DEADLINE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid deadline",
        ));
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let mut group = OwnedGroup {
        child: Some(command.spawn()?),
    };
    let end = Instant::now() + deadline;
    loop {
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // WNOWAIT prevents PID reuse before the owned process group is killed.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                group.child.as_ref().unwrap().id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result != 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                if Instant::now() >= end {
                    group.finish()?;
                    return Ok(Outcome::TimedOut);
                }
                continue;
            }
            if error.raw_os_error() == Some(libc::ECHILD) {
                group.child.take();
            }
            return Err(error);
        }
        if unsafe { info.si_pid() } != 0 {
            return group.finish().map(Outcome::Exited);
        }
        if Instant::now() >= end {
            group.finish()?;
            return Ok(Outcome::TimedOut);
        }
        thread::sleep(POLL.min(end.saturating_duration_since(Instant::now())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestChild(Child);

    impl Drop for TestChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn completes_a_disposable_child() {
        let result =
            unsafe { run(&mut Command::new("/bin/true"), Duration::from_secs(1)) }.unwrap();
        assert!(matches!(result, Outcome::Exited(status) if status.success()));
    }

    #[test]
    fn kills_a_hung_child_within_the_deadline() {
        let start = Instant::now();
        let result = unsafe {
            run(
                Command::new("/bin/sleep").arg("30"),
                Duration::from_millis(100),
            )
        }
        .unwrap();
        assert!(matches!(result, Outcome::TimedOut));
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn kills_a_stopped_child_within_the_deadline() {
        let start = Instant::now();
        let mut child = Command::new("/bin/sh");
        child.args(["-c", "kill -STOP $$; sleep 30"]);
        let result = unsafe { run(&mut child, Duration::from_millis(100)) }.unwrap();
        assert!(matches!(result, Outcome::TimedOut));
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn rejects_invalid_deadlines_before_spawning() {
        for duration in [Duration::ZERO, Duration::from_secs(6)] {
            let error = unsafe { run(&mut Command::new("/nonexistent"), duration) }.unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn cleans_owned_descendants_without_signaling_an_unrelated_process() {
        let mut unrelated = TestChild(Command::new("/bin/sleep").arg("30").spawn().unwrap());
        let file =
            std::env::temp_dir().join(format!("earkey-supervisor-test-{}", std::process::id()));
        let mut shell = Command::new("/bin/sh");
        shell
            .args(["-c", "sleep 30 & printf '%s' $! > \"$1\"; exit 0", "sh"])
            .arg(&file);
        let result = unsafe { run(&mut shell, Duration::from_secs(2)) }.unwrap();
        assert!(matches!(result, Outcome::Exited(status) if status.success()));
        let pid: u32 = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        std::fs::remove_file(&file).unwrap();
        let end = Instant::now() + Duration::from_secs(1);
        loop {
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
            if stat.is_err() || stat.unwrap().split_whitespace().nth(2) == Some("Z") {
                break;
            }
            assert!(Instant::now() < end, "owned descendant is still running");
            thread::sleep(POLL);
        }
        assert!(unrelated.0.try_wait().unwrap().is_none());
    }
}
