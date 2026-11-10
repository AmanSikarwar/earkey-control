use crate::{Command, selinux_context, selinux_enforcing};
use earkey_policy::store::{Store, StoreError};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::process::ExitCode;

const UINPUT: &str = "/dev/uinput";
const LABEL_LIMIT: u64 = 256;

#[cfg(target_arch = "aarch64")]
const _: () = {
    assert!(std::mem::size_of::<libc::input_event>() == 24);
    assert!(std::mem::align_of::<libc::input_event>() == 8);
    assert!(std::mem::offset_of!(libc::input_event, type_) == 16);
    assert!(std::mem::offset_of!(libc::input_event, code) == 18);
    assert!(std::mem::offset_of!(libc::input_event, value) == 20);
};

struct Failure {
    stage: &'static str,
    errno: Option<i32>,
}

impl Failure {
    fn at(stage: &'static str) -> Self {
        Self { stage, errno: None }
    }

    fn from_io(stage: &'static str, error: io::Error) -> Self {
        Self {
            stage,
            errno: error.raw_os_error(),
        }
    }
}

pub(crate) fn run(command: Command) -> ExitCode {
    match check(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("probe={}", failure.stage);
            if let Some(errno) = failure.errno {
                eprintln!("errno={errno}");
            }
            ExitCode::FAILURE
        }
    }
}

fn check(command: Command) -> Result<(), Failure> {
    // SAFETY: These calls take no pointers and do not modify process state.
    let (uid, page_size) = unsafe { (libc::geteuid(), libc::sysconf(libc::_SC_PAGESIZE)) };
    if page_size <= 0 {
        return Err(Failure::at("page_size_unavailable"));
    }

    let context = read_small("/proc/self/attr/current", LABEL_LIMIT).ok();
    let context = context.as_deref().and_then(selinux_context);
    let enforcement = read_small("/sys/fs/selinux/enforce", 2).ok();
    let enforcement = enforcement.as_deref().and_then(selinux_enforcing);
    println!("arch={}", std::env::consts::ARCH);
    println!("page_size={page_size}");
    println!(
        "input_event_size={}",
        std::mem::size_of::<libc::input_event>()
    );
    println!(
        "input_event_type_offset={}",
        std::mem::offset_of!(libc::input_event, type_)
    );
    println!("effective_uid={uid}");
    println!("selinux_context={}", context.unwrap_or("unavailable"));
    println!(
        "selinux_enforcing={}",
        match enforcement {
            Some(true) => "1",
            Some(false) => "0",
            None => "unavailable",
        }
    );

    if command == Command::SelfCheck {
        println!("probe=self_check_ok");
        return Ok(());
    }
    if uid != 0 {
        return Err(Failure::at("root_required"));
    }
    if enforcement != Some(true) || context.is_none() {
        return Err(Failure::at("enforcing_context_required"));
    }

    let store = Store::open().map_err(storage_failure)?;
    if store.has_inactive_marker().map_err(storage_failure)? {
        return Err(Failure::at("stay_inactive"));
    }
    check_uinput_open()?;
    println!("probe=uinput_open_ok");
    Ok(())
}

fn read_small(path: &str, limit: u64) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(data)
}

fn storage_failure(error: StoreError) -> Failure {
    match error {
        StoreError::Io(error) => Failure::from_io("guard_unavailable", error),
        _ => Failure::at("guard_unavailable"),
    }
}

fn check_uinput_open() -> Result<(), Failure> {
    let expected =
        fs::symlink_metadata(UINPUT).map_err(|error| Failure::from_io("uinput_stat", error))?;
    if !is_uinput(&expected) {
        return Err(Failure::at("uinput_identity"));
    }

    let file = OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(UINPUT)
        .map_err(|error| Failure::from_io("uinput_open", error))?;
    let opened = file
        .metadata()
        .map_err(|error| Failure::from_io("uinput_fstat", error))?;
    if !is_uinput(&opened) || opened.dev() != expected.dev() || opened.ino() != expected.ino() {
        return Err(Failure::at("uinput_identity"));
    }
    Ok(())
}

fn is_uinput(metadata: &Metadata) -> bool {
    metadata.file_type().is_char_device() && metadata.rdev() == libc::makedev(10, 223) as u64
}
