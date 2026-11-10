//! Diagnostic entry point: `earkey-worker check`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args != ["check"] {
        eprintln!("usage: earkey-worker check");
        return ExitCode::from(2);
    }
    run()
}

#[cfg(target_os = "android")]
fn run() -> ExitCode {
    let environment = match earkey_worker::environment::read() {
        Ok(environment) => environment,
        Err(error) => {
            println!("environment=error:{:?}", error.kind());
            println!("check=fail");
            return ExitCode::FAILURE;
        }
    };
    let report = earkey_worker::check::run(std::path::Path::new("/"), &environment);
    for (key, value) in &report.lines {
        println!("{key}={value}");
    }
    if report.passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(not(target_os = "android"))]
fn run() -> ExitCode {
    println!("check=android_required");
    ExitCode::FAILURE
}
