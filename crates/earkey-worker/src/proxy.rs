//! uinput proxy for one enrolled accessory, following decision record 0004.
//!
//! The proxy copies only bus, vendor, product and version, uses a fixed name,
//! advertises only the measured control keys and `MSC_SCAN`, and sets no
//! `EV_REP`. Readiness means Android's `system_server` holds the proxy node
//! open, observed within a deadline. Dropping the proxy destroys it.

use earkey_policy::identity::LinuxKey;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// Fixed proxy name; it never matches an accessory layout or the evidence.
pub const PROXY_NAME: &str = "EarKey Control proxy";
const EV_KEY: libc::c_int = 0x01;
const EV_MSC: libc::c_int = 0x04;
const MSC_SCAN: libc::c_int = 0x04;
const NAME_SIZE: usize = 80;
const SYSNAME_LEN: usize = 64;
const POLL: Duration = Duration::from_millis(10);

const fn ioc(dir: u32, nr: u32, size: usize) -> u32 {
    (dir << 30) | ((size as u32 & 0x3fff) << 16) | ((b'U' as u32) << 8) | nr
}

pub const UI_DEV_CREATE: u32 = ioc(0, 1, 0);
pub const UI_DEV_DESTROY: u32 = ioc(0, 2, 0);
pub const UI_DEV_SETUP: u32 = ioc(1, 3, std::mem::size_of::<UinputSetup>());
pub const UI_SET_EVBIT: u32 = ioc(1, 100, 4);
pub const UI_SET_KEYBIT: u32 = ioc(1, 101, 4);
pub const UI_SET_MSCBIT: u32 = ioc(1, 104, 4);
pub const fn ui_get_sysname(len: usize) -> u32 {
    ioc(2, 44, len)
}

/// Kernel `struct uinput_setup`.
#[repr(C)]
pub struct UinputSetup {
    pub id: [u16; 4],
    pub name: [u8; NAME_SIZE],
    pub ff_effects_max: u32,
}

impl UinputSetup {
    pub fn new(input_id: [u16; 4]) -> Self {
        let mut name = [0; NAME_SIZE];
        name[..PROXY_NAME.len()].copy_from_slice(PROXY_NAME.as_bytes());
        Self {
            id: input_id,
            name,
            ff_effects_max: 0,
        }
    }
}

/// A created uinput device. Dropping it destroys the device.
pub struct Proxy {
    file: File,
    pub sysname: String,
}

impl Proxy {
    /// Creates the proxy on `dev/uinput` with the copied input ID and the given keys.
    pub fn create(dev: &Path, input_id: [u16; 4], keys: &[LinuxKey]) -> io::Result<Self> {
        let path = dev.join("uinput");
        let file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_NOCTTY)
            .open(&path)?;
        let metadata = file.metadata()?;
        if !metadata.file_type().is_char_device()
            || metadata.rdev() != crate::inventory::linux_dev(10, 223)
        {
            return Err(invalid("uinput identity"));
        }
        if keys.is_empty() {
            return Err(invalid("no proxy keys"));
        }
        set(&file, UI_SET_EVBIT, EV_KEY)?;
        for key in keys {
            set(&file, UI_SET_KEYBIT, libc::c_int::from(key.0))?;
        }
        set(&file, UI_SET_EVBIT, EV_MSC)?;
        set(&file, UI_SET_MSCBIT, MSC_SCAN)?;
        let setup = UinputSetup::new(input_id);
        // SAFETY: `setup` is a live `struct uinput_setup` of the size encoded in the request.
        check(unsafe { libc::ioctl(file.as_raw_fd(), UI_DEV_SETUP as _, &setup) })?;
        // SAFETY: UI_DEV_CREATE takes no argument.
        check(unsafe { libc::ioctl(file.as_raw_fd(), UI_DEV_CREATE as _) })?;
        let mut proxy = Self {
            file,
            sysname: String::new(),
        };
        let mut name = [0u8; SYSNAME_LEN];
        // SAFETY: `name` holds SYSNAME_LEN writable bytes, the size encoded in the request.
        check(unsafe {
            libc::ioctl(
                proxy.file.as_raw_fd(),
                ui_get_sysname(SYSNAME_LEN) as _,
                name.as_mut_ptr(),
            )
        })?;
        proxy.sysname = name
            .split(|&byte| byte == 0)
            .next()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .filter(|name| valid_sysname(name))
            .ok_or_else(|| invalid("uinput sysname"))?
            .to_owned();
        Ok(proxy)
    }
}

impl Proxy {
    pub(crate) fn file(&self) -> &File {
        &self.file
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        // SAFETY: UI_DEV_DESTROY takes no argument; the descriptor is still open.
        unsafe { libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY as _) };
    }
}

/// A proxy node that Android has opened.
#[derive(Debug, Eq, PartialEq)]
pub struct Ready {
    pub event: String,
    pub elapsed: Duration,
}

/// Waits until the proxy's event node exists in `dev` and `system_server` holds it open.
pub fn wait_ready(
    sys: &Path,
    dev: &Path,
    proc: &Path,
    sysname: &str,
    deadline: Duration,
) -> io::Result<Ready> {
    if !valid_sysname(sysname) {
        return Err(invalid("uinput sysname"));
    }
    let start = Instant::now();
    loop {
        if let Some(event) = event_node(sys, sysname)? {
            let node = dev.join("input").join(&event);
            if node_matches(sys, sysname, &event, &node)? && system_server_holds(proc, &node)? {
                return Ok(Ready {
                    event,
                    elapsed: start.elapsed(),
                });
            }
        }
        if start.elapsed() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "proxy not opened by system_server",
            ));
        }
        thread::sleep(POLL);
    }
}

/// True once the proxy's event node is gone from `dev`.
pub fn node_removed(dev: &Path, event: &str) -> bool {
    fs::symlink_metadata(dev.join("input").join(event)).is_err()
}

fn event_node(sys: &Path, sysname: &str) -> io::Result<Option<String>> {
    let directory = sys.join("devices/virtual/input").join(sysname);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut events = Vec::new();
    for entry in entries {
        let name = entry?.file_name();
        if let Some(name) = name.to_str()
            && name
                .strip_prefix("event")
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        {
            events.push(name.to_owned());
        }
    }
    match events.as_slice() {
        [] => Ok(None),
        [event] => Ok(Some(event.clone())),
        _ => Err(invalid("proxy has several event nodes")),
    }
}

fn node_matches(sys: &Path, sysname: &str, event: &str, node: &Path) -> io::Result<bool> {
    let dev_attr = sys
        .join("devices/virtual/input")
        .join(sysname)
        .join(event)
        .join("dev");
    let text = match fs::read_to_string(dev_attr) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let (major, minor) = text
        .trim_end()
        .split_once(':')
        .ok_or_else(|| invalid("dev"))?;
    let parse = |field: &str| field.parse::<u32>().map_err(|_| invalid("dev"));
    let expected = crate::inventory::linux_dev(parse(major)?, parse(minor)?);
    match fs::symlink_metadata(node) {
        Ok(metadata) => Ok(metadata.file_type().is_char_device() && metadata.rdev() == expected),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// True if a process whose command line is exactly `system_server` has `node` open.
pub fn system_server_holds(proc: &Path, node: &Path) -> io::Result<bool> {
    for entry in fs::read_dir(proc)? {
        let entry = entry?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.bytes().all(|b| b.is_ascii_digit()))
        {
            continue;
        }
        let process: PathBuf = entry.path();
        let Ok(cmdline) = fs::read(process.join("cmdline")) else {
            continue;
        };
        if cmdline.split(|&byte| byte == 0).next() != Some(b"system_server") {
            continue;
        }
        let Ok(descriptors) = fs::read_dir(process.join("fd")) else {
            continue;
        };
        for descriptor in descriptors.flatten() {
            if fs::read_link(descriptor.path()).is_ok_and(|target| target == node) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn valid_sysname(name: &str) -> bool {
    name.strip_prefix("input")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn set(file: &File, request: u32, value: libc::c_int) -> io::Result<()> {
    // SAFETY: UI_SET_*BIT takes an integer argument by value.
    check(unsafe { libc::ioctl(file.as_raw_fd(), request as _, value) })
}

fn check(result: libc::c_int) -> io::Result<()> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}
