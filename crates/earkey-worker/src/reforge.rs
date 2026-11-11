//! Opt-in entry point for the 11 November reforge passthrough check.

use std::process::ExitCode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Check,
    ProxyCheck,
    Supervise,
    Worker,
}

fn command(args: &[&str]) -> Option<Command> {
    match args {
        ["check"] => Some(Command::Check),
        ["proxy-check"] => Some(Command::ProxyCheck),
        ["supervise"] => Some(Command::Supervise),
        ["worker"] => Some(Command::Worker),
        _ => None,
    }
}

pub(super) fn main(args: &[&str]) -> ExitCode {
    let Some(command) = command(args) else {
        eprintln!("usage: earkey-reforge-passthrough check | proxy-check | supervise");
        return ExitCode::from(2);
    };
    match run(command) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("diagnostic=error:{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "android"))]
fn run(_: Command) -> std::io::Result<bool> {
    Err(std::io::Error::other("android_required"))
}

#[cfg(any(target_os = "android", test))]
fn installed_module_is_inactive(gate: earkey_supervisor::Gate, supervisor: Option<&str>) -> bool {
    gate != earkey_supervisor::Gate::OPEN
        && matches!(
            supervisor.and_then(|status| status.lines().next()),
            Some("phase=Latched" | "phase=Inactive(Paused)" | "phase=Inactive(BootDisabled)")
        )
}

#[cfg(target_os = "android")]
fn run(command: Command) -> std::io::Result<bool> {
    use super::{STOP, dumpsys_input, on_terminate};
    use earkey_policy::identity::Evidence;
    use earkey_policy::store::{Runtime, Store};
    use earkey_supervisor::process::Lock;
    use earkey_supervisor::{Gate, store::read_gate};
    use earkey_worker::{check, environment, proxy_check, session};
    use std::io;
    use std::path::Path;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    const DIRECTORY: &str = "/data/adb/earkey_control_reforge_check";
    const SESSION: Duration = Duration::from_secs(120);
    let root = Path::new("/");
    // SAFETY: The handler only stores to an atomic.
    unsafe {
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            libc::signal(signal, on_terminate as *const () as libc::sighandler_t);
        }
    }
    let selinux = std::fs::read_to_string("/proc/self/attr/current")?;
    if selinux.trim_end_matches(['\0', '\n']) != "u:r:ksu:s0"
        || std::fs::read_to_string("/sys/fs/selinux/enforce")?.trim() != "1"
        // SAFETY: _SC_PAGESIZE is a valid selector and takes no other arguments.
        || unsafe { libc::sysconf(libc::_SC_PAGESIZE) } != 4096
    {
        return Err(io::Error::other("root_context_mismatch"));
    }
    let environment = environment::read()?;
    if !Evidence::XiaomiPixelosReforge20261111.matches_environment(&environment.as_environment()) {
        return Err(io::Error::other("reference_environment_mismatch"));
    }
    let installed = Store::open().map_err(|_| io::Error::other("installed_store_unavailable"))?;
    let supervisor = installed
        .read_runtime(Runtime::Supervisor)
        .map_err(|_| io::Error::other("installed_supervisor_unavailable"))?;
    if !installed_module_is_inactive(read_gate(&installed)?, supervisor.as_deref()) {
        return Err(io::Error::other("installed_supervisor_gate_mismatch"));
    }
    let directory = Path::new(DIRECTORY);
    let diagnostic =
        Store::open_dir(directory).map_err(|_| io::Error::other("diagnostic_store_unavailable"))?;
    let gate = || {
        let inactive = diagnostic
            .has_inactive_marker()
            .map_err(|_| io::Error::other("diagnostic_gate_unavailable"))?;
        Ok(Gate {
            paused: inactive || STOP.load(Ordering::SeqCst),
            ..Gate::OPEN
        })
    };
    if gate()? != Gate::OPEN {
        return Err(io::Error::other("diagnostic_gate_closed"));
    }

    let _lock = match command {
        Command::ProxyCheck | Command::Supervise => {
            Some(Lock::acquire(&directory.join("supervisor.lock"))?)
        }
        _ => None,
    };
    let report = match command {
        Command::Check => check::run(root, &environment),
        Command::ProxyCheck => {
            proxy_check::run(root, &environment, Duration::from_secs(45), dumpsys_input)
        }
        Command::Supervise => {
            let exe = std::env::current_exe()?;
            let limits = earkey_supervisor::Limits {
                restart: SESSION + Duration::from_secs(1),
                ..session::LIMITS
            };
            // SAFETY: Only the adapter reaps children in this process.
            unsafe { session::supervise(exe, vec!["worker".into()], SESSION, limits, gate) }
        }
        Command::Worker => {
            // SAFETY: getppid has no arguments or memory preconditions.
            let parent = unsafe { libc::getppid() };
            if std::fs::read_link(format!("/proc/{parent}/exe"))? != std::env::current_exe()? {
                return Err(io::Error::other("supervisor_required"));
            }
            return Ok(session::worker(
                root,
                &environment,
                &session::Selection::Passthrough,
                gate,
                &STOP,
                &mut io::stdout().lock(),
                &mut io::stderr().lock(),
            ));
        }
    };
    for (key, value) in report.lines {
        println!("{key}={value}");
    }
    Ok(report.passed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use earkey_supervisor::Gate;

    #[test]
    fn only_the_diagnostic_commands_are_accepted() {
        for (name, allowed) in [
            ("check", Command::Check),
            ("proxy-check", Command::ProxyCheck),
            ("supervise", Command::Supervise),
            ("worker", Command::Worker),
        ] {
            assert_eq!(command(&[name]), Some(allowed));
            assert_eq!(command(&[name, "block-all"]), None);
        }
        for name in ["enroll", "run", "daemon", "apply", "resume", "clear-latch"] {
            assert_eq!(command(&[name]), None);
        }
    }

    #[test]
    fn the_installed_module_must_be_closed_and_confirmed_inactive() {
        let latched = Gate {
            latched: true,
            ..Gate::OPEN
        };
        let paused = Gate {
            paused: true,
            ..Gate::OPEN
        };
        let status = |phase: &str| format!("phase={phase}\nfailures=0\nlast_fault=none\n");
        assert!(installed_module_is_inactive(
            latched,
            Some(&status("Latched"))
        ));
        assert!(installed_module_is_inactive(
            paused,
            Some(&status("Inactive(Paused)"))
        ));
        assert!(installed_module_is_inactive(
            Gate {
                boot_disabled: true,
                ..Gate::OPEN
            },
            Some(&status("Inactive(BootDisabled)"))
        ));
        assert!(!installed_module_is_inactive(
            Gate::OPEN,
            Some(&status("Inactive(Paused)"))
        ));
        for phase in ["Running", "Starting", "Inactive(Ready)", "Stopping { .. }"] {
            assert!(
                !installed_module_is_inactive(paused, Some(&status(phase))),
                "{phase}"
            );
        }
        assert!(!installed_module_is_inactive(paused, None));
    }
}
