use earkey_policy::identity::{Evidence, VerifiedEnrollment};
use earkey_policy::store::{RecoveryOperation, Store};
use earkey_worker::control::{ControlError, apply, recovery, set_enabled, status};
use earkey_worker::selection::Selection;
use std::fs::{self, DirBuilder};
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const SERIAL: &str = "SYNTHETIC-ENROLLMENT";

struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "earkey-control-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn get<'a>(lines: &'a [(&'static str, String)], key: &str) -> Vec<&'a str> {
    lines
        .iter()
        .filter(|(k, _)| *k == key)
        .map(|(_, v)| v.as_str())
        .collect()
}

#[test]
fn apply_enable_and_recovery_keep_generations_and_recovery_state() {
    let dir = Dir::new();
    let store = Store::open_dir(&dir.0).unwrap();
    let empty = status(&store);
    assert_eq!(get(&empty, "enrollments"), ["0"]);
    assert_eq!(get(&empty, "saved_generation"), ["0"]);
    assert_eq!(get(&empty, "enabled"), ["false"]);
    assert!(get(&empty, "control").is_empty());
    assert!(matches!(
        apply(&store, &Selection::BlockAll, true),
        Err(ControlError::NotEnrolled)
    ));

    let evidence = Evidence::XiaomiPixelosReforge20261110;
    store
        .enroll(&VerifiedEnrollment::from_reviewed_measurement(evidence, SERIAL).unwrap())
        .unwrap();
    assert_eq!(apply(&store, &Selection::BlockAll, false).unwrap(), 1);
    let lines = status(&store);
    assert_eq!(
        get(&lines, "enrolled_evidence"),
        ["xiaomi_pixelos_reforge20261110"]
    );
    assert_eq!(get(&lines, "enabled"), ["false"]);
    assert_eq!(
        get(&lines, "profile_blocked_groups"),
        ["Center,VolumeUp,VolumeDown"]
    );
    let controls = get(&lines, "control");
    assert_eq!(controls.len(), 3);
    assert!(controls[0].starts_with("center:Play or pause|"));
    assert!(controls[0].ends_with("|Reject an incoming call"));
    assert!(controls[1].starts_with("volume-up:Raise volume|"));
    assert!(controls[2].starts_with("volume-down:Lower volume|"));
    for control in controls {
        let name = control.split_once(':').unwrap().0;
        assert!(Selection::parse(name).is_some(), "{name}");
    }

    assert_eq!(set_enabled(&store, true).unwrap(), 2);
    assert_eq!(get(&status(&store), "enabled"), ["true"]);
    let center = Selection::parse("center").unwrap();
    assert_eq!(apply(&store, &center, true).unwrap(), 3);
    assert_eq!(get(&status(&store), "profile_blocked_groups"), ["Center"]);

    assert_eq!(recovery(&store, RecoveryOperation::Pause).unwrap(), 4);
    assert_eq!(apply(&store, &Selection::BlockAll, true).unwrap(), 5);
    let lines = status(&store);
    assert_eq!(get(&lines, "paused"), ["true"]);
    assert_eq!(get(&lines, "saved_generation"), ["5"]);

    assert_eq!(
        recovery(&store, RecoveryOperation::DisableNextBoot).unwrap(),
        6
    );
    let lines = status(&store);
    assert_eq!(get(&lines, "stay_inactive"), ["true"]);
    assert_eq!(get(&lines, "boot_disabled"), ["true"]);
    assert_eq!(
        recovery(&store, RecoveryOperation::EnableNextBoot).unwrap(),
        7
    );
    assert_eq!(recovery(&store, RecoveryOperation::Resume).unwrap(), 8);
    let lines = status(&store);
    assert_eq!(get(&lines, "stay_inactive"), ["false"]);
    assert_eq!(get(&lines, "paused"), ["false"]);
    assert!(!lines.iter().any(|(_, v)| v.contains(SERIAL)));
    assert_eq!(get(&lines, "worker"), ["none"]);
    store
        .write_runtime(
            earkey_policy::store::Runtime::Worker,
            "state=waiting_accessory\nactive_generation=0\n",
        )
        .unwrap();
    store
        .write_runtime(
            earkey_policy::store::Runtime::Supervisor,
            "phase=Running\nfailures=0\nlast_fault=none\n",
        )
        .unwrap();
    let lines = status(&store);
    assert_eq!(
        get(&lines, "worker"),
        ["state=waiting_accessory", "active_generation=0"]
    );
    assert_eq!(
        get(&lines, "supervisor"),
        ["phase=Running", "failures=0", "last_fault=none"]
    );
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod live {
    use super::*;
    use earkey_worker::control::enroll;
    use earkey_worker::environment::{Observed, parse};
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;
    use std::path::Path;

    const LAYOUT_TEXT: &str = "key 115 VOLUME_UP\n";
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

    #[test]
    fn enrollment_needs_one_matching_live_candidate_and_layout() {
        let dir = Dir::new();
        let store = Store::open_dir(&dir.0).unwrap();
        let root = Root::new();
        assert!(matches!(
            enroll(&store, &root.0, &reforge()),
            Err(ControlError::Layout(_))
        ));
        root.write(&root.0.join("system/usr/keylayout/Generic.kl"), LAYOUT_TEXT);
        assert!(matches!(
            enroll(&store, &root.0, &reforge()),
            Err(ControlError::NoCandidate)
        ));
        root.earphones("1-1", "event9", 73);
        // The fake Generic.kl digest differs from the evidence, so nothing enrolls.
        assert!(matches!(
            enroll(&store, &root.0, &reforge()),
            Err(ControlError::NoCandidate)
        ));
        assert!(store.enrollments().unwrap().is_empty());
    }
}
