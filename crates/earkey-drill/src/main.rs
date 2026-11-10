//! Bounded no-grab supervision drill. Workers never open an input device, and
//! the drill never writes policy.

use std::process::ExitCode;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [command] = args.as_slice() else {
        return usage();
    };
    match command.to_str() {
        Some(command @ ("drill" | "worker" | "worker-ignore-term" | "worker-stall")) => {
            run(command)
        }
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: earkey-drill drill | worker | worker-ignore-term | worker-stall");
    ExitCode::from(2)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn run(command: &str) -> ExitCode {
    linux::run(command)
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn run(_: &str) -> ExitCode {
    eprintln!("drill=linux_required");
    ExitCode::FAILURE
}
