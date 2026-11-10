#![cfg(any(target_os = "linux", target_os = "android"))]

use earkey_policy::identity::NodeInstance;
use earkey_worker::device::{
    EVIOCGID, Opened, byte_bits, eviocgbit, eviocgkey, eviocgname, eviocgprop,
};
use earkey_worker::inventory::Observation;
use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn ioctl_numbers_match_the_kernel_encoding() {
    assert_eq!(EVIOCGID, 0x8008_4502);
    assert_eq!(eviocgname(256), 0x8100_4506);
    assert_eq!(eviocgprop(4), 0x8004_4509);
    assert_eq!(eviocgkey(96), 0x8060_4518);
    assert_eq!(eviocgbit(0, 4), 0x8004_4520);
    assert_eq!(eviocgbit(1, 96), 0x8060_4521);
    assert_eq!(eviocgbit(4, 1), 0x8001_4524);
}

#[test]
fn byte_bitmaps_decode_little_endian_bits() {
    assert_eq!(byte_bits(&[0x13]), [0, 1, 4]);
    assert_eq!(byte_bits(&[0x10]), [4]);
    let mut keys = [0u8; 96];
    for key in [0x71u16, 0x72, 0x73, 0x8b, 0xa3, 0xa4, 0xa5, 0x246] {
        keys[key as usize / 8] |= 1 << (key % 8);
    }
    assert_eq!(
        byte_bits(&keys),
        [0x71, 0x72, 0x73, 0x8b, 0xa3, 0xa4, 0xa5, 0x246]
    );
    assert!(byte_bits(&[0; 4]).is_empty());
}

struct Dev(PathBuf);

impl Dev {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "earkey-device-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("input")).unwrap();
        Self(root)
    }

    fn mknod(&self, name: &str, major: u32, minor: u32) -> PathBuf {
        let path = self.0.join("input").join(name);
        let c = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: The path is NUL-terminated and valid for the call.
        let result = unsafe {
            libc::mknod(
                c.as_ptr(),
                libc::S_IFCHR | 0o600,
                libc::makedev(major, minor),
            )
        };
        assert_eq!(
            result,
            0,
            "mknod requires CAP_MKNOD: {}",
            io::Error::last_os_error()
        );
        path
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn observation(instance: NodeInstance) -> Observation {
    Observation {
        event: "event5".into(),
        instance,
        usb_hid: None,
        usb_serial: None,
        usb_driver: None,
        is_virtual: false,
        input_id: [0; 4],
        name: String::new(),
        event_types: vec![],
        keys: vec![],
        misc: vec![],
        properties: vec![],
    }
}

fn instance(path: &PathBuf) -> NodeInstance {
    let metadata = fs::symlink_metadata(path).unwrap();
    NodeInstance {
        filesystem_device: metadata.dev(),
        inode: metadata.ino(),
        character_device: metadata.rdev(),
    }
}

#[test]
fn a_non_evdev_or_different_node_is_rejected() {
    let dev = Dev::new();
    let null = dev.mknod("event5", 1, 3);
    let matching = observation(instance(&null));
    assert!(Opened::open(&dev.0, &matching).is_err());

    let other = NodeInstance {
        inode: matching.instance.inode + 1,
        ..matching.instance
    };
    assert_eq!(
        Opened::open(&dev.0, &observation(other))
            .err()
            .map(|e| e.kind()),
        Some(io::ErrorKind::InvalidData)
    );

    fs::remove_file(&null).unwrap();
    fs::write(&null, b"").unwrap();
    assert!(Opened::open(&dev.0, &observation(instance(&null))).is_err());
}
