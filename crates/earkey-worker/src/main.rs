//! Diagnostic entry point: `earkey-worker check <layout-sha256>`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [command, layout] = args.as_slice() else {
        return usage();
    };
    if command != "check" {
        return usage();
    }
    run(layout)
}

fn usage() -> ExitCode {
    eprintln!("usage: earkey-worker check <layout-sha256>");
    ExitCode::from(2)
}

#[cfg(target_os = "android")]
fn run(layout: &str) -> ExitCode {
    let environment = match earkey_worker::environment::read() {
        Ok(environment) => environment,
        Err(error) => {
            println!("environment=error:{:?}", error.kind());
            println!("check=fail");
            return ExitCode::FAILURE;
        }
    };
    let report = earkey_worker::check::run(std::path::Path::new("/"), &environment, layout);
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
fn run(_: &str) -> ExitCode {
    println!("check=android_required");
    ExitCode::FAILURE
}
