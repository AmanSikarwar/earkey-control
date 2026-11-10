//! Diagnostic entry point:
//! `earkey-worker check | proxy-check | passthrough [selection] | supervise <seconds> [selection]`,
//! where selection is `passthrough`, `block-all` or a comma list of `center`, `volume-up`, `volume-down`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let command = match args.as_slice() {
        [command @ ("check" | "proxy-check")] => Command::Plain(command),
        ["passthrough"] => Command::Passthrough("passthrough"),
        ["passthrough", selection] => Command::Passthrough(selection),
        ["supervise", seconds, rest @ ..] if rest.len() <= 1 => match seconds.parse::<u64>() {
            Ok(seconds) if earkey_worker_sessions().contains(&seconds) => {
                Command::Supervise(seconds, rest.first().copied().unwrap_or("passthrough"))
            }
            _ => return usage(),
        },
        _ => return usage(),
    };
    run(command)
}

enum Command<'a> {
    Plain(&'a str),
    Passthrough(&'a str),
    Supervise(u64, &'a str),
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: earkey-worker check | proxy-check | passthrough [selection] | supervise <10-600> [selection]"
    );
    ExitCode::from(2)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn earkey_worker_sessions() -> std::ops::RangeInclusive<u64> {
    earkey_worker::session::SESSION_SECONDS
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn earkey_worker_sessions() -> std::ops::RangeInclusive<u64> {
    10..=600
}

#[cfg(target_os = "android")]
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "android")]
extern "C" fn on_terminate(_: libc::c_int) {
    STOP.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(target_os = "android")]
fn run(command: Command<'_>) -> ExitCode {
    use earkey_policy::store::Store;
    use earkey_supervisor::process::Lock;
    use std::path::Path;
    use std::time::Duration;

    const LOCK: &str = "/data/adb/earkey_control/supervisor.lock";
    let store_gate = || {
        let store = Store::open().map_err(|error| std::io::Error::other(format!("{error:?}")))?;
        earkey_supervisor::store::read_gate(&store)
    };
    let root = Path::new("/");
    if let Command::Supervise(seconds, selection) = command {
        if earkey_worker::session::Selection::parse(selection).is_none() {
            return usage();
        }
        let lock = match Lock::acquire(Path::new(LOCK)) {
            Ok(lock) => lock,
            Err(error) => {
                println!("supervise_lock=error:{:?}", error.kind());
                println!("supervise=fail");
                return ExitCode::FAILURE;
            }
        };
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(error) => {
                println!("supervise_exe=error:{:?}", error.kind());
                return ExitCode::FAILURE;
            }
        };
        // SAFETY: This single-threaded process reaps children only through the adapter.
        let report = unsafe {
            earkey_worker::session::supervise(
                exe,
                vec!["passthrough".into(), selection.into()],
                Duration::from_secs(seconds),
                earkey_worker::session::LIMITS,
                store_gate,
            )
        };
        for (key, value) in &report.lines {
            println!("{key}={value}");
        }
        drop(lock);
        println!(
            "supervise_lock_removed={}",
            std::fs::remove_file(LOCK).is_ok()
        );
        return if report.passed {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    let (command, selection) = match command {
        Command::Plain(command) => (command, None),
        Command::Passthrough(text) => match earkey_worker::session::Selection::parse(text) {
            Some(selection) => ("passthrough", Some(selection)),
            None => return usage(),
        },
        Command::Supervise(..) => unreachable!(),
    };

    let environment = match earkey_worker::environment::read() {
        Ok(environment) => environment,
        Err(error) => {
            println!("environment=error:{:?}", error.kind());
            println!("check=fail");
            return ExitCode::FAILURE;
        }
    };
    if let Some(selection) = selection {
        // SAFETY: The handler only stores to an atomic, which is async-signal-safe.
        unsafe {
            libc::signal(
                libc::SIGTERM,
                on_terminate as *const () as libc::sighandler_t,
            )
        };
        let passed = earkey_worker::session::worker(
            root,
            &environment,
            &selection,
            store_gate,
            &STOP,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        );
        return if passed {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    let report = if command == "check" {
        earkey_worker::check::run(root, &environment)
    } else {
        earkey_worker::proxy_check::run(root, &environment, Duration::from_secs(45), dumpsys_input)
    };
    for (key, value) in &report.lines {
        println!("{key}={value}");
    }
    if report.passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Runs `dumpsys input` with a 10 s deadline and a 4 MiB output limit.
#[cfg(target_os = "android")]
fn dumpsys_input() -> std::io::Result<String> {
    use std::io::{self, Read};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const LIMIT: u64 = 4 * 1024 * 1024;
    let mut child = Command::new("/system/bin/dumpsys")
        .arg("input")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().ok_or(io::ErrorKind::BrokenPipe)?;
    let reader = std::thread::spawn(move || {
        let mut text = Vec::new();
        let mut limited = stdout.take(LIMIT);
        limited.read_to_end(&mut text).map(|_| text)
    });
    let end = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if Instant::now() >= end {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let text = reader
        .join()
        .map_err(|_| io::Error::other("dumpsys reader panicked"))??;
    match status {
        Some(status) if status.success() => Ok(String::from_utf8_lossy(&text).into_owned()),
        Some(_) => Err(io::Error::other("dumpsys failed")),
        None => Err(io::ErrorKind::TimedOut.into()),
    }
}

#[cfg(not(target_os = "android"))]
fn run(command: Command<'_>) -> ExitCode {
    match command {
        Command::Plain(command) => println!("{command}=android_required"),
        Command::Passthrough(selection) => println!("passthrough_{selection}=android_required"),
        Command::Supervise(seconds, selection) => {
            println!("supervise_{seconds}_{selection}=android_required")
        }
    }
    ExitCode::FAILURE
}
