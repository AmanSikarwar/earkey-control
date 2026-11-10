//! Diagnostic entry point: `earkey-worker check` or `earkey-worker proxy-check`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [command @ ("check" | "proxy-check")] => run(command),
        _ => {
            eprintln!("usage: earkey-worker check | proxy-check");
            ExitCode::from(2)
        }
    }
}

#[cfg(target_os = "android")]
fn run(command: &str) -> ExitCode {
    use std::path::Path;
    use std::time::Duration;

    let environment = match earkey_worker::environment::read() {
        Ok(environment) => environment,
        Err(error) => {
            println!("environment=error:{:?}", error.kind());
            println!("check=fail");
            return ExitCode::FAILURE;
        }
    };
    let root = Path::new("/");
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
fn run(_: &str) -> ExitCode {
    println!("check=android_required");
    ExitCode::FAILURE
}
