use std::ffi::{OsStr, OsString};
use std::process::ExitCode;

#[cfg(target_os = "android")]
mod android;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    SelfCheck,
    ProbeUinput,
}

fn command(mut args: impl Iterator<Item = OsString>) -> Option<Command> {
    let action = args.next()?;
    if args.next().is_some() {
        return None;
    }
    if action == OsStr::new("self-check") {
        Some(Command::SelfCheck)
    } else if action == OsStr::new("probe-uinput") {
        Some(Command::ProbeUinput)
    } else {
        None
    }
}

fn main() -> ExitCode {
    let Some(command) = command(std::env::args_os().skip(1)) else {
        eprintln!("usage: earkey-probe self-check | probe-uinput");
        return ExitCode::from(2);
    };
    run(command)
}

#[cfg(target_os = "android")]
fn run(command: Command) -> ExitCode {
    android::run(command)
}

#[cfg(not(target_os = "android"))]
fn run(_: Command) -> ExitCode {
    eprintln!("probe=android_required");
    ExitCode::FAILURE
}

#[cfg(any(target_os = "android", test))]
fn selinux_context(bytes: &[u8]) -> Option<&str> {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\0").unwrap_or(bytes);
    let label = std::str::from_utf8(bytes).ok()?;
    if label
        .splitn(4, ':')
        .filter(|field| !field.is_empty())
        .count()
        != 4
        || !label.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b',')
        })
    {
        return None;
    }
    Some(label)
}

#[cfg(any(target_os = "android", test))]
fn selinux_enforcing(bytes: &[u8]) -> Option<bool> {
    match bytes {
        b"1" | b"1\n" => Some(true),
        b"0" | b"0\n" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_two_explicit_probe_commands() {
        let parse = |args: &[&str]| command(args.iter().map(OsString::from));
        assert_eq!(parse(&["self-check"]), Some(Command::SelfCheck));
        assert_eq!(parse(&["probe-uinput"]), Some(Command::ProbeUinput));
        for args in [
            &[][..],
            &["grab"],
            &["probe-uinput", "/dev/input/event0"],
            &["self-check", "--force"],
        ] {
            assert_eq!(parse(args), None);
        }
    }

    #[test]
    fn context_accepts_kernel_terminators_and_rejects_display_injection() {
        assert_eq!(selinux_context(b"u:r:ksu:s0\0\n"), Some("u:r:ksu:s0"));
        assert_eq!(
            selinux_context(b"u:r:example_service:s0:c1,c256"),
            Some("u:r:example_service:s0:c1,c256")
        );
        for invalid in [
            &b"u::ksu:s0"[..],
            b"u:r:ksu",
            b"u:r:ksu:s0\nstatus=ok",
            b"u:r:ksu:s0\x1b",
            b"u:r:ksu:s0\0\0",
            b"\xff",
        ] {
            assert_eq!(selinux_context(invalid), None);
        }
    }

    #[test]
    fn enforcing_requires_an_exact_kernel_value() {
        assert_eq!(selinux_enforcing(b"1\n"), Some(true));
        assert_eq!(selinux_enforcing(b"0"), Some(false));
        for invalid in [&b""[..], b"10", b"1\n0", b"enforcing"] {
            assert_eq!(selinux_enforcing(invalid), None);
        }
    }
}
