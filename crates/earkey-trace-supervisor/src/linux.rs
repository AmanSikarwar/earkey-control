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

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    #[test]
    fn traces_an_owned_child_info_ioctl_using_arm64_registers() {
        use std::ffi::c_void;

        const PTRACE_TRACEME: libc::c_int = 0;
        const PTRACE_SYSCALL: libc::c_int = 24;
        const PTRACE_SETOPTIONS: libc::c_int = 0x4200;
        const PTRACE_GETREGSET: libc::c_int = 0x4204;
        const PTRACE_O_TRACESYSGOOD: usize = 1;
        const PTRACE_O_EXITKILL: usize = 1 << 20;
        const NT_PRSTATUS: usize = 1;
        const SYS_IOCTL: u64 = 29;
        const GET_INFO: u64 = 0x80104b02;

        struct Tracee(libc::pid_t);

        impl Tracee {
            fn wait(&self, deadline: Instant) -> io::Result<i32> {
                loop {
                    let mut status = 0;
                    let result = unsafe { libc::waitpid(self.0, &mut status, libc::WNOHANG) };
                    if result == self.0 {
                        return Ok(status);
                    }
                    if result < 0 {
                        let error = io::Error::last_os_error();
                        if error.kind() != io::ErrorKind::Interrupted {
                            return Err(error);
                        }
                    }
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(io::ErrorKind::TimedOut, "tracee deadline"));
                    }
                    thread::sleep(Duration::from_millis(1));
                }
            }

            fn reaped(&mut self) {
                self.0 = -1;
            }
        }

        impl Drop for Tracee {
            fn drop(&mut self) {
                if self.0 <= 0 {
                    return;
                }
                unsafe { libc::kill(self.0, libc::SIGKILL) };
                loop {
                    let mut status = 0;
                    let result = unsafe { libc::waitpid(self.0, &mut status, 0) };
                    if result == self.0
                        || (result < 0
                            && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD))
                    {
                        break;
                    }
                    if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
                    {
                        break;
                    }
                }
            }
        }

        fn ptrace(
            request: libc::c_int,
            pid: libc::pid_t,
            addr: usize,
            data: usize,
        ) -> io::Result<()> {
            let result =
                unsafe { libc::ptrace(request, pid, addr as *mut c_void, data as *mut c_void) };
            if result == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        fn arm64_regs(pid: libc::pid_t) -> io::Result<[u64; 34]> {
            let mut regs = [0_u64; 34];
            let mut iov = libc::iovec {
                iov_base: regs.as_mut_ptr().cast(),
                iov_len: std::mem::size_of_val(&regs),
            };
            ptrace(
                PTRACE_GETREGSET,
                pid,
                NT_PRSTATUS,
                (&mut iov as *mut libc::iovec) as usize,
            )?;
            if iov.iov_len != std::mem::size_of_val(&regs) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "short arm64 register set",
                ));
            }
            Ok(regs)
        }

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork: {}", io::Error::last_os_error());
        if pid == 0 {
            unsafe {
                if libc::ptrace(
                    PTRACE_TRACEME,
                    0,
                    std::ptr::null_mut::<c_void>(),
                    std::ptr::null_mut::<c_void>(),
                ) == -1
                {
                    libc::_exit(120);
                }
                libc::raise(libc::SIGSTOP);
                let result = libc::syscall(libc::SYS_ioctl, -1, GET_INFO as libc::c_ulong, 0);
                let error = *libc::__errno_location();
                libc::_exit(i32::from(!(result == -1 && error == libc::EBADF)));
            }
        }

        let mut tracee = Tracee(pid);
        let deadline = Instant::now() + Duration::from_secs(2);
        let initial = tracee.wait(deadline).unwrap();
        assert!(libc::WIFSTOPPED(initial));
        assert_eq!(libc::WSTOPSIG(initial), libc::SIGSTOP);
        ptrace(
            PTRACE_SETOPTIONS,
            pid,
            0,
            PTRACE_O_TRACESYSGOOD | PTRACE_O_EXITKILL,
        )
        .unwrap();

        let mut syscall_entry = true;
        let mut pending_info = false;
        let mut info_failures = 0;
        loop {
            ptrace(PTRACE_SYSCALL, pid, 0, 0).unwrap();
            let status = tracee.wait(deadline).unwrap();
            if libc::WIFEXITED(status) {
                tracee.reaped();
                assert_eq!(libc::WEXITSTATUS(status), 0);
                break;
            }
            assert!(
                libc::WIFSTOPPED(status),
                "unexpected tracee status {status:#x}"
            );
            assert_eq!(
                libc::WSTOPSIG(status),
                libc::SIGTRAP | 0x80,
                "unexpected signal stop {status:#x}"
            );
            let regs = arm64_regs(pid).unwrap();
            if syscall_entry {
                if regs[8] == SYS_IOCTL && regs[0] as i32 == -1 && regs[1] == GET_INFO {
                    assert!(!pending_info, "overlapping query");
                    pending_info = true;
                }
            } else if pending_info {
                assert_eq!(regs[0] as i64, -(libc::EBADF as i64));
                pending_info = false;
                info_failures += 1;
            }
            syscall_entry = !syscall_entry;
        }
        assert!(!pending_info, "query had no matching syscall exit");
        assert_eq!(info_failures, 1);
    }
}
