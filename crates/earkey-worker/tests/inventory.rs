use earkey_worker::inventory::{bits, scan};
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SERIAL: &str = "SYNTHETIC-ENROLLMENT";
const KEYS: [u16; 8] = [0x71, 0x72, 0x73, 0x8b, 0xa3, 0xa4, 0xa5, 0x246];
const USB_INPUT: &str = "devices/platform/soc/usb1/1-1/1-1:1.3/0003:0020:0B21.0001/input/input7";

struct Tree(PathBuf);

impl Tree {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "earkey-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("sys/class/input")).unwrap();
        fs::create_dir_all(root.join("sys/devices/virtual")).unwrap();
        fs::create_dir_all(root.join("dev/input")).unwrap();
        Self(root)
    }

    fn sys(&self) -> PathBuf {
        self.0.join("sys")
    }

    fn dev(&self) -> PathBuf {
        self.0.join("dev")
    }

    fn write(&self, path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("{text}\n")).unwrap();
    }

    fn input(&self, relative: &str, event: &str, minor: u32, name: &str, keys: &[u16]) -> PathBuf {
        let input = self.sys().join(relative);
        self.write(&input.join("name"), name);
        for (field, value) in [
            ("bustype", "0003"),
            ("vendor", "0020"),
            ("product", "0b21"),
            ("version", "0201"),
        ] {
            self.write(&input.join("id").join(field), value);
        }
        self.write(&input.join("capabilities/ev"), "13");
        self.write(&input.join("capabilities/key"), &bitmap(keys));
        self.write(&input.join("capabilities/msc"), "10");
        self.write(&input.join("properties"), "0");
        self.write(&input.join(event).join("dev"), &format!("13:{minor}"));
        symlink(
            input.join(event),
            self.sys().join("class/input").join(event),
        )
        .unwrap();
        input
    }

    fn usb(&self, input: &Path) {
        let interface = input.parent().unwrap().parent().unwrap().parent().unwrap();
        let device = interface.parent().unwrap();
        for (field, value) in [("bInterfaceNumber", "03"), ("bInterfaceClass", "03")] {
            self.write(&interface.join(field), value);
        }
        for (field, value) in [
            ("idVendor", "0020"),
            ("idProduct", "0b21"),
            ("bcdDevice", "0100"),
            ("serial", SERIAL),
        ] {
            self.write(&device.join(field), value);
        }
        let driver = self.sys().join("bus/usb/drivers/usbhid");
        fs::create_dir_all(&driver).unwrap();
        symlink(driver, interface.join("driver")).unwrap();
    }

    fn earphones(&self, event: &str, minor: u32) {
        let input = self.input(
            USB_INPUT,
            event,
            minor,
            "Generic Xiaomi Type-C Earphones 1",
            &KEYS,
        );
        self.usb(&input);
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Formats set bits as sysfs `unsigned long` words, most significant first.
fn bitmap(set: &[u16]) -> String {
    let width = usize::BITS as usize;
    let words = set
        .iter()
        .map(|&bit| bit as usize / width + 1)
        .max()
        .unwrap_or(1);
    let mut values = vec![0u64; words];
    for &bit in set {
        values[bit as usize / width] |= 1 << (bit as usize % width);
    }
    values
        .iter()
        .rev()
        .map(|value| format!("{value:x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn bitmaps_decode_least_significant_word_last() {
    assert_eq!(bits("13").unwrap(), [0, 1, 4]);
    assert_eq!(bits(&bitmap(&KEYS)).unwrap(), KEYS);
    assert_eq!(bits("0").unwrap(), Vec::<u16>::new());
    for invalid in ["", " ", "x", "-1", "1 0x2"] {
        assert!(bits(invalid).is_err(), "{invalid:?}");
    }
    assert!(bits(&vec!["0"; 65].join(" ")).is_err());
}

#[test]
fn a_regular_file_is_not_an_input_node() {
    let tree = Tree::new();
    tree.earphones("event5", 69);
    fs::write(tree.dev().join("input/event5"), b"").unwrap();
    assert_eq!(
        scan(&tree.sys(), &tree.dev()).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn too_many_nodes_fail_the_scan() {
    let tree = Tree::new();
    for number in 0..65 {
        symlink(
            "missing",
            tree.sys().join(format!("class/input/event{number}")),
        )
        .unwrap();
    }
    assert!(scan(&tree.sys(), &tree.dev()).is_err());
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod nodes {
    use super::*;
    use earkey_policy::identity::{Environment, Evidence, LinuxKey, VerifiedEnrollment};
    use earkey_policy::policy::Policy;

    const LAYOUT: &str = "4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773";
    const REQUEST: &str = include_str!("../../../tests/fixtures/identity/reference-policy.json");
    const ENVIRONMENT: Environment<'static> = Environment {
        model: "M2101K6P",
        device: "sweetin",
        api: 37,
        rom: "PixelOS_sweet-17.0-20261030-2116",
        build_id: "CP2A.260605.016",
        incremental: "1793394994",
        kernel_release: "4.14.357-openela-VantomKernel",
        kernel_build: "#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026",
    };
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    fn mknod(tree: &Tree, event: &str, minor: u32) {
        let path =
            CString::new(tree.dev().join("input").join(event).as_os_str().as_bytes()).unwrap();
        // SAFETY: The path is NUL-terminated and valid for the call.
        let result = unsafe {
            libc::mknod(
                path.as_ptr(),
                libc::S_IFCHR | 0o600,
                libc::makedev(13, minor),
            )
        };
        assert_eq!(
            result,
            0,
            "mknod requires CAP_MKNOD: {}",
            io::Error::last_os_error()
        );
    }

    #[test]
    fn reference_shaped_node_matches_the_policy_evidence() {
        let tree = Tree::new();
        tree.earphones("event5", 69);
        mknod(&tree, "event5", 69);
        let observations = scan(&tree.sys(), &tree.dev()).unwrap();
        assert_eq!(observations.len(), 1);
        let observation = &observations[0];
        assert_eq!(observation.event, "event5");
        assert_eq!(observation.input_id, [3, 0x20, 0x0b21, 0x0201]);
        assert_eq!(observation.event_types, [0, 1, 4]);
        assert_eq!(observation.keys, KEYS.map(LinuxKey));
        assert_eq!(observation.misc, [4]);
        assert!(observation.properties.is_empty());
        assert_eq!(observation.usb_serial.as_deref(), Some(SERIAL));
        assert_eq!(observation.usb_driver.as_deref(), Some("usbhid"));
        assert!(!observation.is_virtual);
        assert_eq!(observation.instance.character_device, libc::makedev(13, 69));

        let enrollment =
            VerifiedEnrollment::from_reviewed_measurement(Evidence::XiaomiPixelos20261107, SERIAL)
                .unwrap();
        let policy = Policy::parse(REQUEST.as_bytes(), &[enrollment]).unwrap();
        let targets = policy
            .targets(&ENVIRONMENT, &[observation.candidate(LAYOUT)])
            .unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].instance(), observation.instance);
    }

    #[test]
    fn virtual_and_non_usb_nodes_carry_no_usb_identity() {
        let tree = Tree::new();
        tree.input(
            "devices/virtual/input/input9",
            "event9",
            73,
            "Generic Xiaomi Type-C Earphones 1",
            &KEYS,
        );
        mknod(&tree, "event9", 73);
        tree.input(
            "devices/platform/gpio-keys/input/input0",
            "event0",
            64,
            "gpio-keys",
            &[0x72, 0x73, 0x74],
        );
        mknod(&tree, "event0", 64);
        let observations = scan(&tree.sys(), &tree.dev()).unwrap();
        let virtual_node = observations.iter().find(|o| o.event == "event9").unwrap();
        assert!(virtual_node.is_virtual);
        assert!(virtual_node.usb_hid.is_none());
        let keys = observations.iter().find(|o| o.event == "event0").unwrap();
        assert!(!keys.is_virtual);
        assert!(keys.usb_hid.is_none());
        assert!(keys.usb_serial.is_none());

        let enrollment =
            VerifiedEnrollment::from_reviewed_measurement(Evidence::XiaomiPixelos20261107, SERIAL)
                .unwrap();
        let policy = Policy::parse(REQUEST.as_bytes(), &[enrollment]).unwrap();
        let candidates: Vec<_> = observations.iter().map(|o| o.candidate(LAYOUT)).collect();
        assert!(
            policy
                .targets(&ENVIRONMENT, &candidates)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn mismatched_symlinked_or_unreadable_nodes_fail_closed() {
        let tree = Tree::new();
        tree.earphones("event5", 69);
        mknod(&tree, "event5", 68);
        assert!(scan(&tree.sys(), &tree.dev()).is_err());

        fs::remove_file(tree.dev().join("input/event5")).unwrap();
        mknod(&tree, "event4", 69);
        symlink("event4", tree.dev().join("input/event5")).unwrap();
        assert!(scan(&tree.sys(), &tree.dev()).is_err());

        fs::remove_file(tree.dev().join("input/event5")).unwrap();
        mknod(&tree, "event5", 69);
        assert_eq!(scan(&tree.sys(), &tree.dev()).unwrap().len(), 1);
        fs::remove_file(tree.sys().join(USB_INPUT).join("name")).unwrap();
        assert!(scan(&tree.sys(), &tree.dev()).is_err());
    }

    #[test]
    fn changed_usb_shape_drops_usb_identity() {
        let tree = Tree::new();
        tree.earphones("event5", 69);
        mknod(&tree, "event5", 69);
        let hid = tree
            .sys()
            .join(USB_INPUT)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        fs::rename(&hid, hid.with_file_name("0003:0020:0B21")).unwrap();
        fs::remove_file(tree.sys().join("class/input/event5")).unwrap();
        let moved = hid
            .with_file_name("0003:0020:0B21")
            .join("input/input7/event5");
        symlink(moved, tree.sys().join("class/input/event5")).unwrap();
        let observations = scan(&tree.sys(), &tree.dev()).unwrap();
        assert!(observations[0].usb_hid.is_none());
    }
}
