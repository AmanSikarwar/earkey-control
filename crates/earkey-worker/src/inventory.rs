//! Read-only input inventory from sysfs. It never opens an input device.
//!
//! Any node that cannot be fully observed fails the whole scan, so an
//! unreadable duplicate cannot hide an ambiguity from the matcher.

use earkey_policy::identity::{Candidate, LinuxKey, NodeInstance, UsbHid};
use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

const MAX_NODES: usize = 64;
const MAX_ATTRIBUTE: u64 = 4096;
const MAX_NAME: usize = 256;
const MAX_BITMAP_WORDS: usize = 64;

/// Complete observations for one `eventN` node.
#[derive(Debug, Eq, PartialEq)]
pub struct Observation {
    pub event: String,
    pub instance: NodeInstance,
    pub usb_hid: Option<UsbHid>,
    pub usb_serial: Option<String>,
    pub usb_driver: Option<String>,
    pub is_virtual: bool,
    pub input_id: [u16; 4],
    pub name: String,
    pub event_types: Vec<u16>,
    pub keys: Vec<LinuxKey>,
    pub misc: Vec<u16>,
    pub properties: Vec<u16>,
}

impl Observation {
    /// Borrows this observation as a matcher candidate.
    pub fn candidate<'a>(&'a self, layout_sha256: &'a str) -> Candidate<'a> {
        Candidate {
            instance: self.instance,
            usb_hid: self.usb_hid,
            usb_serial: self.usb_serial.as_deref(),
            usb_driver: self.usb_driver.as_deref(),
            is_virtual: self.is_virtual,
            input_id: self.input_id,
            name: &self.name,
            layout_sha256,
            event_types: &self.event_types,
            keys: &self.keys,
            misc: &self.misc,
            properties: &self.properties,
        }
    }
}

/// Observes every `eventN` node under `sys/class/input`, checking each against `dev/input`.
pub fn scan(sys: &Path, dev: &Path) -> io::Result<Vec<Observation>> {
    let mut events = Vec::new();
    for entry in fs::read_dir(sys.join("class/input"))? {
        let name = entry?.file_name();
        let Some(name) = name.to_str() else { continue };
        if name
            .strip_prefix("event")
            .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
        {
            events.push(name.to_owned());
        }
        if events.len() > MAX_NODES {
            return Err(invalid("too many input nodes"));
        }
    }
    events.sort();
    let virtual_root = fs::canonicalize(sys.join("devices/virtual"))?;
    events
        .into_iter()
        .map(|event| observe(sys, dev, &virtual_root, event))
        .collect()
}

fn observe(sys: &Path, dev: &Path, virtual_root: &Path, event: String) -> io::Result<Observation> {
    let real = fs::canonicalize(sys.join("class/input").join(&event))?;
    if real.file_name().and_then(|name| name.to_str()) != Some(event.as_str()) {
        return Err(invalid("event path changed"));
    }
    let input = real.parent().ok_or_else(|| invalid("event has no input"))?;
    let (major, minor) = device_numbers(&attribute(&real.join("dev"))?)?;
    let instance = node(&dev.join("input").join(&event), major, minor)?;

    let name = attribute(&input.join("name"))?;
    if name.len() > MAX_NAME || name.chars().any(char::is_control) {
        return Err(invalid("input name"));
    }
    let id = |field| hex_u16(&attribute(&input.join("id").join(field))?);
    let input_id = [
        id("bustype")?,
        id("vendor")?,
        id("product")?,
        id("version")?,
    ];
    let bitmap = |path: PathBuf| bits(&attribute(&path)?);
    let capabilities = input.join("capabilities");

    let (usb_hid, usb_serial, usb_driver) = usb_ancestry(input)?;
    Ok(Observation {
        instance,
        usb_hid,
        usb_serial,
        usb_driver,
        is_virtual: real.starts_with(virtual_root),
        input_id,
        name,
        event_types: bitmap(capabilities.join("ev"))?,
        keys: bitmap(capabilities.join("key"))?
            .into_iter()
            .map(LinuxKey)
            .collect(),
        misc: bitmap(capabilities.join("msc"))?,
        properties: bitmap(input.join("properties"))?,
        event,
    })
}

type Usb = (Option<UsbHid>, Option<String>, Option<String>);

/// Accepts only `usb-device/interface/BBBB:VVVV:PPPP.N/input/inputM`.
fn usb_ancestry(input: &Path) -> io::Result<Usb> {
    let parent = |path: &Path| path.parent().map(Path::to_path_buf);
    let Some(hid) = parent(input)
        .filter(|path| path.file_name().is_some_and(|name| name == "input"))
        .and_then(|path| parent(&path))
    else {
        return Ok((None, None, None));
    };
    let Some(ids) = hid
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(hid_ids)
    else {
        return Ok((None, None, None));
    };
    let Some(interface) = parent(&hid).filter(|path| path.join("bInterfaceNumber").is_file())
    else {
        return Ok((None, None, None));
    };
    let Some(device) = parent(&interface).filter(|path| path.join("idVendor").is_file()) else {
        return Ok((None, None, None));
    };
    let usb = UsbHid {
        vendor: hex_u16(&attribute(&device.join("idVendor"))?)?,
        product: hex_u16(&attribute(&device.join("idProduct"))?)?,
        revision: hex_u16(&attribute(&device.join("bcdDevice"))?)?,
        interface: hex_u8(&attribute(&interface.join("bInterfaceNumber"))?)?,
        interface_class: hex_u8(&attribute(&interface.join("bInterfaceClass"))?)?,
        hid_bus: ids[0],
        hid_vendor: ids[1],
        hid_product: ids[2],
    };
    let serial = optional(&device.join("serial"))?;
    let driver = match fs::read_link(interface.join("driver")) {
        Ok(target) => target
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok((Some(usb), serial, driver))
}

fn hid_ids(name: &str) -> Option<[u16; 3]> {
    let (ids, instance) = name.split_once('.')?;
    if instance.is_empty() || !instance.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut fields = ids.split(':');
    let mut next = || {
        let field = fields.next().filter(|field| field.len() == 4)?;
        u16::from_str_radix(field, 16).ok()
    };
    let ids = [next()?, next()?, next()?];
    fields.next().is_none().then_some(ids)
}

fn node(path: &Path, major: u32, minor: u32) -> io::Result<NodeInstance> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_char_device() || metadata.rdev() != linux_dev(major, minor) {
        return Err(invalid("device node does not match sysfs"));
    }
    Ok(NodeInstance {
        filesystem_device: metadata.dev(),
        inode: metadata.ino(),
        character_device: metadata.rdev(),
    })
}

/// Linux `makedev` encoding.
pub(crate) fn linux_dev(major: u32, minor: u32) -> u64 {
    let (major, minor) = (u64::from(major), u64::from(minor));
    ((major & 0xffff_f000) << 32)
        | ((major & 0xfff) << 8)
        | ((minor & 0xffff_ff00) << 12)
        | (minor & 0xff)
}

fn device_numbers(text: &str) -> io::Result<(u32, u32)> {
    let (major, minor) = text.split_once(':').ok_or_else(|| invalid("dev"))?;
    let parse = |field: &str| field.parse().map_err(|_| invalid("dev"));
    Ok((parse(major)?, parse(minor)?))
}

/// Parses a sysfs bitmap: hex `unsigned long` words, most significant first.
pub fn bits(text: &str) -> io::Result<Vec<u16>> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() || words.len() > MAX_BITMAP_WORDS {
        return Err(invalid("bitmap"));
    }
    let width = usize::BITS;
    let mut set = Vec::new();
    for (index, word) in words.iter().rev().enumerate() {
        let value = u64::from_str_radix(word, 16).map_err(|_| invalid("bitmap"))?;
        if width < 64 && value >> width != 0 {
            return Err(invalid("bitmap"));
        }
        for bit in 0..width {
            if value >> bit & 1 == 1 {
                let position = index as u32 * width + bit;
                set.push(u16::try_from(position).map_err(|_| invalid("bitmap"))?);
            }
        }
    }
    Ok(set)
}

fn hex_u16(text: &str) -> io::Result<u16> {
    u16::from_str_radix(text, 16).map_err(|_| invalid("hex attribute"))
}

fn hex_u8(text: &str) -> io::Result<u8> {
    u8::from_str_radix(text, 16).map_err(|_| invalid("hex attribute"))
}

fn optional(path: &Path) -> io::Result<Option<String>> {
    match attribute(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Reads one bounded UTF-8 attribute without its trailing newline.
fn attribute(path: &Path) -> io::Result<String> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(MAX_ATTRIBUTE + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_ATTRIBUTE {
        return Err(invalid("oversized attribute"));
    }
    Ok(text.strip_suffix('\n').unwrap_or(&text).to_owned())
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}
