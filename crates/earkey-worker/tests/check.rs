#![cfg(any(target_os = "linux", target_os = "android"))]

use earkey_worker::check::run;
use earkey_worker::environment::{Observed, parse};
use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const LAYOUT: &str = "4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773";
const KEY_BITMAP: &str = "40 0 0 0 0 0 0 3800000800 e000000000000 0";
const NAME: &str = "Generic Xiaomi Type-C Earphones 1";

struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "earkey-check-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("sys/class/input")).unwrap();
        fs::create_dir_all(root.join("sys/devices/virtual")).unwrap();
        fs::create_dir_all(root.join("dev/input")).unwrap();
        Self(root)
    }

    fn write(&self, path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("{text}\n")).unwrap();
    }

    fn earphones(&self, usb: &str, event: &str, minor: u32) {
        let device = self.0.join("sys/devices/platform/soc").join(usb);
        let interface = device.join(format!("{usb}:1.3"));
        let input = interface.join("0003:0020:0B21.0001/input/input10");
        self.write(&input.join("name"), NAME);
        for (field, value) in [
            ("bustype", "0003"),
            ("vendor", "0020"),
            ("product", "0b21"),
            ("version", "0201"),
        ] {
            self.write(&input.join("id").join(field), value);
        }
        self.write(&input.join("capabilities/ev"), "13");
        self.write(&input.join("capabilities/key"), KEY_BITMAP);
        self.write(&input.join("capabilities/msc"), "10");
        self.write(&input.join("properties"), "0");
        self.write(&input.join(event).join("dev"), &format!("13:{minor}"));
        symlink(
            input.join(event),
            self.0.join("sys/class/input").join(event),
        )
        .unwrap();
        for (field, value) in [("bInterfaceNumber", "03"), ("bInterfaceClass", "03")] {
            self.write(&interface.join(field), value);
        }
        for (field, value) in [
            ("idVendor", "0020"),
            ("idProduct", "0b21"),
            ("bcdDevice", "0100"),
            ("serial", "SYNTHETIC-ENROLLMENT"),
        ] {
            self.write(&device.join(field), value);
        }
        let driver = self.0.join("sys/bus/usb/drivers/usbhid");
        fs::create_dir_all(&driver).unwrap();
        symlink(driver, interface.join("driver")).unwrap();

        let path =
            CString::new(self.0.join("dev/input").join(event).as_os_str().as_bytes()).unwrap();
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
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn environment(kernel: &str) -> Observed {
    let props = |name: &str| {
        Some(
            match name {
                "ro.product.model" => "M2101K6P",
                "ro.product.device" => "sweetin",
                "ro.build.version.sdk" => "37",
                "ro.custom.version" => "PixelOS_sweet-17.0-20261030-2116",
                "ro.build.id" => "CP2A.260605.016",
                "ro.build.version.incremental" => "1793394994",
                _ => return None,
            }
            .to_owned(),
        )
    };
    parse(props, kernel).unwrap()
}

fn reforge() -> Observed {
    environment(
        "Linux version 4.14.357-reforge-g9b21e3df6c0a (b@h) (c) #2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026",
    )
}

fn lines(root: &Root, environment: &Observed, layout: &str) -> (Vec<String>, bool) {
    let report = run(&root.0, environment, layout);
    (
        report
            .lines
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect(),
        report.passed,
    )
}

#[test]
fn reference_earphones_reach_the_open_step() {
    let root = Root::new();
    root.earphones("1-1", "event9", 73);
    let (lines, passed) = lines(&root, &reforge(), LAYOUT);
    assert!(!passed);
    assert_eq!(
        lines[..6],
        [
            "evidence=xiaomi_pixelos_reforge20261110",
            "inventory_nodes=1",
            "enrollment=observed",
            "layout=generic",
            "targets=1",
            lines[5].as_str(),
        ]
    );
    assert!(lines[5].starts_with("open=error:"), "{lines:?}");
    assert_eq!(lines.last().unwrap(), "check=fail");
    assert!(!lines.iter().any(|line| line.contains("SYNTHETIC")));
}

#[test]
fn device_specific_layout_or_configuration_stops_the_check() {
    for file in [
        "vendor/usr/keylayout/Vendor_0020_Product_0b21.kl",
        "odm/usr/keylayout/Vendor_0020_Product_0b21_Version_0201.kl",
        "data/system/devices/keylayout/Generic_Xiaomi_Type-C_Earphones_1.kl",
        "system/usr/idc/Vendor_0020_Product_0b21.idc",
    ] {
        let root = Root::new();
        root.earphones("1-1", "event9", 73);
        root.write(&root.0.join(file), "");
        let (lines, passed) = lines(&root, &reforge(), LAYOUT);
        assert!(!passed);
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("layout=override_present")),
            "{file}: {lines:?}"
        );
    }
}

#[test]
fn missing_duplicate_wrong_kernel_or_bad_digest_stops_the_check() {
    let root = Root::new();
    assert!(
        lines(&root, &reforge(), LAYOUT)
            .0
            .contains(&"enrollment=none".to_owned())
    );

    root.earphones("1-1", "event9", 73);
    root.earphones("1-2", "event10", 74);
    assert!(
        lines(&root, &reforge(), LAYOUT)
            .0
            .contains(&"enrollment=ambiguous".to_owned())
    );

    let other =
        environment("Linux version 4.14.357-other (b@h) (c) #3 SMP PREEMPT Tue Nov 10 2026");
    assert_eq!(
        lines(&root, &other, LAYOUT).0,
        ["evidence=none", "check=fail"]
    );

    for bad in ["", "abc", &LAYOUT.to_uppercase(), &format!("{LAYOUT}0")] {
        assert_eq!(
            lines(&root, &reforge(), bad).0,
            ["layout=invalid_digest", "check=fail"]
        );
    }
    assert!(
        lines(&root, &reforge(), &"0".repeat(64))
            .0
            .contains(&"enrollment=none".to_owned())
    );
}
