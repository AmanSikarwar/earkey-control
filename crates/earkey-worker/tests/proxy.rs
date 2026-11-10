#![cfg(any(target_os = "linux", target_os = "android"))]

use earkey_policy::identity::{Environment, Evidence, LinuxKey, VerifiedEnrollment};
use earkey_policy::policy::Policy;
use earkey_worker::inventory::scan;
use earkey_worker::proxy::{
    PROXY_NAME, Proxy, UI_DEV_CREATE, UI_DEV_DESTROY, UI_DEV_SETUP, UI_SET_EVBIT, UI_SET_KEYBIT,
    UI_SET_MSCBIT, UinputSetup, system_server_holds, ui_get_sysname, wait_ready,
};
use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const INPUT_ID: [u16; 4] = [3, 0x20, 0x0b21, 0x0201];
const KEYS: [LinuxKey; 3] = [LinuxKey(0x72), LinuxKey(0x73), LinuxKey(0xa4)];

struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "earkey-proxy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("dev/input")).unwrap();
        Self(root)
    }

    fn mknod(&self, path: &Path, major: u32, minor: u32) {
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
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn uinput_requests_and_setup_layout_match_the_kernel() {
    assert_eq!(UI_DEV_CREATE, 0x5501);
    assert_eq!(UI_DEV_DESTROY, 0x5502);
    assert_eq!(std::mem::size_of::<UinputSetup>(), 92);
    assert_eq!(UI_DEV_SETUP, 0x405c_5503);
    assert_eq!(UI_SET_EVBIT, 0x4004_5564);
    assert_eq!(UI_SET_KEYBIT, 0x4004_5565);
    assert_eq!(UI_SET_MSCBIT, 0x4004_5568);
    assert_eq!(ui_get_sysname(64), 0x8040_552c);
    let setup = UinputSetup::new(INPUT_ID);
    assert_eq!(setup.id, INPUT_ID);
    assert_eq!(&setup.name[..PROXY_NAME.len()], PROXY_NAME.as_bytes());
    assert!(setup.name[PROXY_NAME.len()..].iter().all(|&byte| byte == 0));
    assert_eq!(setup.ff_effects_max, 0);
}

fn fake_proxy(root: &Root, sysname: &str, event: &str, minor: u32) -> PathBuf {
    let dir = root
        .0
        .join("sys/devices/virtual/input")
        .join(sysname)
        .join(event);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("dev"), format!("13:{minor}\n")).unwrap();
    let node = root.0.join("dev/input").join(event);
    root.mknod(&node, 13, minor);
    node
}

fn fake_process(root: &Root, pid: u32, cmdline: &[u8], fds: &[&Path]) {
    let process = root.0.join("proc").join(pid.to_string());
    fs::create_dir_all(process.join("fd")).unwrap();
    fs::write(process.join("cmdline"), cmdline).unwrap();
    for (number, target) in fds.iter().enumerate() {
        symlink(target, process.join("fd").join(number.to_string())).unwrap();
    }
}

#[test]
fn readiness_requires_the_node_and_a_system_server_descriptor() {
    let root = Root::new();
    let (sys, dev, proc) = (root.0.join("sys"), root.0.join("dev"), root.0.join("proc"));
    fs::create_dir_all(&proc).unwrap();
    let short = Duration::from_millis(50);
    assert_eq!(
        wait_ready(&sys, &dev, &proc, "input12", short)
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );

    let node = fake_proxy(&root, "input12", "event12", 76);
    fake_process(&root, 100, b"zygote64\0", &[&node]);
    fake_process(&root, 101, b"system_server_x\0", &[&node]);
    assert!(!system_server_holds(&proc, &node).unwrap());
    assert_eq!(
        wait_ready(&sys, &dev, &proc, "input12", short)
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );

    fake_process(
        &root,
        102,
        b"system_server\0",
        &[Path::new("/dev/null"), &node],
    );
    let ready = wait_ready(&sys, &dev, &proc, "input12", Duration::from_secs(1)).unwrap();
    assert_eq!(ready.event, "event12");

    for bad in ["", "input", "event12", "input1/../input12", "../input12"] {
        assert!(
            wait_ready(&sys, &dev, &proc, bad, short).is_err(),
            "{bad:?}"
        );
    }
}

#[test]
fn readiness_rejects_a_mismatched_or_duplicated_node() {
    let root = Root::new();
    let (sys, dev, proc) = (root.0.join("sys"), root.0.join("dev"), root.0.join("proc"));
    let node = root.0.join("dev/input/event12");
    let dir = root.0.join("sys/devices/virtual/input/input12/event12");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("dev"), "13:76\n").unwrap();
    root.mknod(&node, 13, 77);
    fake_process(&root, 102, b"system_server\0", &[&node]);
    assert_eq!(
        wait_ready(&sys, &dev, &proc, "input12", Duration::from_millis(50))
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );

    fs::create_dir_all(root.0.join("sys/devices/virtual/input/input12/event13")).unwrap();
    assert_eq!(
        wait_ready(&sys, &dev, &proc, "input12", Duration::from_millis(50))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}

/// Creates a real uinput device in the test kernel. Run in the documented
/// container with `--device /dev/uinput`; never on the reference phone.
#[test]
#[ignore = "needs /dev/uinput and CAP_MKNOD; see docs/build.md"]
fn real_proxy_is_virtual_measured_and_never_targeted() {
    let proxy = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let input = Path::new("/sys/devices/virtual/input").join(&proxy.sysname);
    assert_eq!(
        fs::read_to_string(input.join("name")).unwrap(),
        format!("{PROXY_NAME}\n")
    );
    for (field, value) in [
        ("bustype", "0003"),
        ("vendor", "0020"),
        ("product", "0b21"),
        ("version", "0201"),
    ] {
        assert_eq!(
            fs::read_to_string(input.join("id").join(field))
                .unwrap()
                .trim(),
            value
        );
    }

    let root = Root::new();
    for entry in fs::read_dir("/sys/class/input").unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if let Some(number) = name.strip_prefix("event") {
            let dev =
                fs::read_to_string(Path::new("/sys/class/input").join(&name).join("dev")).unwrap();
            let (major, minor) = dev.trim().split_once(':').unwrap();
            assert_eq!(major, "13", "{number}");
            root.mknod(
                &root.0.join("dev/input").join(&name),
                13,
                minor.parse().unwrap(),
            );
        }
    }
    let observations = scan(Path::new("/sys"), &root.0.join("dev")).unwrap();
    let observed = observations.iter().find(|o| o.name == PROXY_NAME).unwrap();
    assert!(observed.is_virtual);
    assert!(observed.usb_hid.is_none());
    assert_eq!(observed.input_id, INPUT_ID);
    assert_eq!(observed.event_types, [0, 1, 4]);
    assert_eq!(observed.keys, KEYS);
    assert_eq!(observed.misc, [4]);

    let environment = Environment {
        model: "M2101K6P",
        device: "sweetin",
        api: 37,
        rom: "PixelOS_sweet-17.0-20261030-2116",
        build_id: "CP2A.260605.016",
        incremental: "1793394994",
        kernel_release: "4.14.357-reforge-g9b21e3df6c0a",
        kernel_build: "#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026",
    };
    let evidence = Evidence::XiaomiPixelosReforge20261110;
    let layout = "4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773";
    let candidates: Vec<_> = observations.iter().map(|o| o.candidate(layout)).collect();
    assert!(
        candidates
            .iter()
            .all(|c| VerifiedEnrollment::from_observation(evidence, &environment, c).is_none())
    );
    let enrollment =
        VerifiedEnrollment::from_reviewed_measurement(evidence, "SYNTHETIC-ENROLLMENT").unwrap();
    let request = include_str!("../../../tests/fixtures/identity/reference-policy.json")
        .replace("xiaomi_pixelos20261107", "xiaomi_pixelos_reforge20261110");
    let policy = Policy::parse(request.as_bytes(), &[enrollment]).unwrap();
    assert!(
        policy
            .targets(&environment, &candidates)
            .unwrap()
            .is_empty()
    );

    drop(proxy);
    assert!(!input.exists());
}
