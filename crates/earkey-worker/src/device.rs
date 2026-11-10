//! Opens an inventoried input node and re-reads its identity from the descriptor.
//!
//! The node is opened read-only and is never read or grabbed here. The opened
//! observation feeds `Policy::revalidate`, and `pressed_keys` supports the
//! neutral-state check before any later acquisition.

use crate::inventory::Observation;
use earkey_policy::identity::{Candidate, LinuxKey, NodeInstance};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::Path;

const EV_KEY: u32 = 0x01;
const EV_MSC: u32 = 0x04;
const EV_CNT: usize = 0x20;
const KEY_CNT: usize = 0x300;
const MSC_CNT: usize = 0x08;
const PROP_CNT: usize = 0x20;
const NAME_LEN: usize = 256;

/// Linux generic `_IOC(_IOC_READ, 'E', nr, size)`, as used on arm64.
pub const fn eviocg(nr: u32, size: usize) -> u32 {
    (2 << 30) | ((size as u32 & 0x3fff) << 16) | ((b'E' as u32) << 8) | nr
}

pub const EVIOCGID: u32 = eviocg(0x02, 8);
pub const fn eviocgname(len: usize) -> u32 {
    eviocg(0x06, len)
}
pub const fn eviocgprop(len: usize) -> u32 {
    eviocg(0x09, len)
}
pub const fn eviocgkey(len: usize) -> u32 {
    eviocg(0x18, len)
}
pub const fn eviocgbit(event: u32, len: usize) -> u32 {
    eviocg(0x20 + event, len)
}

/// Bit indices set in a little-endian evdev byte bitmap.
pub fn byte_bits(bytes: &[u8]) -> Vec<u16> {
    let mut set = Vec::new();
    for (index, byte) in bytes.iter().enumerate() {
        for bit in 0..8 {
            if byte >> bit & 1 == 1 {
                set.push((index * 8 + bit) as u16);
            }
        }
    }
    set
}

/// A read-only descriptor for an inventoried node, with its descriptor-side identity.
pub struct Opened {
    file: File,
    pub instance: NodeInstance,
    pub input_id: [u16; 4],
    pub name: String,
    pub event_types: Vec<u16>,
    pub keys: Vec<LinuxKey>,
    pub misc: Vec<u16>,
    pub properties: Vec<u16>,
}

impl Opened {
    /// Opens `dev/input/<event>` and requires the opened node to be the inventoried instance.
    pub fn open(dev: &Path, observed: &Observation) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_NOCTTY)
            .open(dev.join("input").join(&observed.event))?;
        let metadata = file.metadata()?;
        let instance = NodeInstance {
            filesystem_device: metadata.dev(),
            inode: metadata.ino(),
            character_device: metadata.rdev(),
        };
        if !metadata.file_type().is_char_device() || instance != observed.instance {
            return Err(invalid("opened node is not the inventoried instance"));
        }
        let mut id = [0u16; 4];
        ioctl(&file, EVIOCGID, id.as_mut_ptr().cast(), 8)?;
        let mut name = [0u8; NAME_LEN];
        let length = ioctl(&file, eviocgname(NAME_LEN), name.as_mut_ptr(), NAME_LEN)?;
        let name = name[..length]
            .split(|&byte| byte == 0)
            .next()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .filter(|name| !name.chars().any(char::is_control))
            .ok_or_else(|| invalid("input name"))?
            .to_owned();
        Ok(Self {
            event_types: bitmap(&file, eviocgbit(0, EV_CNT / 8), EV_CNT)?,
            keys: bitmap(&file, eviocgbit(EV_KEY, KEY_CNT / 8), KEY_CNT)?
                .into_iter()
                .map(LinuxKey)
                .collect(),
            misc: bitmap(&file, eviocgbit(EV_MSC, MSC_CNT.div_ceil(8)), MSC_CNT)?,
            properties: bitmap(&file, eviocgprop(PROP_CNT / 8), PROP_CNT)?,
            file,
            instance,
            input_id: id,
            name,
        })
    }

    /// Descriptor-side identity with USB ancestry from the matching inventory observation.
    pub fn candidate<'a>(
        &'a self,
        observed: &'a Observation,
        layout_sha256: &'a str,
    ) -> Candidate<'a> {
        Candidate {
            instance: self.instance,
            input_id: self.input_id,
            name: &self.name,
            event_types: &self.event_types,
            keys: &self.keys,
            misc: &self.misc,
            properties: &self.properties,
            ..observed.candidate(layout_sha256)
        }
    }

    /// Keys the kernel currently reports as pressed. Empty means neutral.
    pub fn pressed_keys(&self) -> io::Result<Vec<LinuxKey>> {
        Ok(bitmap(&self.file, eviocgkey(KEY_CNT / 8), KEY_CNT)?
            .into_iter()
            .map(LinuxKey)
            .collect())
    }
}

fn bitmap(file: &File, request: u32, bits: usize) -> io::Result<Vec<u16>> {
    let mut bytes = vec![0u8; bits.div_ceil(8)];
    let length = ioctl(file, request, bytes.as_mut_ptr(), bytes.len())?;
    Ok(byte_bits(&bytes[..length]))
}

/// Issues a read ioctl into `len` bytes and returns the byte count the kernel reported.
fn ioctl(file: &File, request: u32, buffer: *mut u8, len: usize) -> io::Result<usize> {
    // SAFETY: `buffer` points to at least `len` writable bytes and `request` encodes that size.
    let result = unsafe { libc::ioctl(file.as_raw_fd(), request as _, buffer) };
    let length = usize::try_from(result).map_err(|_| io::Error::last_os_error())?;
    if request == EVIOCGID {
        return Ok(len);
    }
    if length > len {
        return Err(invalid("ioctl length"));
    }
    Ok(length)
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}
