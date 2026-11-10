#![cfg(any(target_os = "linux", target_os = "android"))]
//! End-to-end production loop in the test kernel: a real uinput device stands
//! in for the earphones behind a fake USB sysfs tree, and a helper thread plays
//! Android by mirroring each proxy into the fake roots and holding it open as a
//! fake `system_server`. Run in the documented container; never on the phone.

use earkey_engine::Event;
use earkey_policy::identity::{Evidence, VerifiedEnrollment};
use earkey_policy::store::Store;
use earkey_worker::control::apply;
use earkey_worker::environment::{Observed, parse};
use earkey_worker::forward::{Limits, encode};
use earkey_worker::proxy::{
    PROXY_NAME, UI_DEV_CREATE, UI_DEV_DESTROY, UI_DEV_SETUP, UI_SET_EVBIT, UI_SET_KEYBIT,
    UI_SET_MSCBIT, UinputSetup, ui_get_sysname,
};
use earkey_worker::run::{Context, run};
use earkey_worker::selection::Selection;
use std::ffi::CString;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const SERIAL: &str = "SYNTHETIC-ENROLLMENT";
const LAYOUT: &str = "4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773";
const INPUT_ID: [u16; 4] = [3, 0x20, 0x0b21, 0x0201];
const EARPHONE_KEYS: [u16; 8] = [0x71, 0x72, 0x73, 0x8b, 0xa3, 0xa4, 0xa5, 0x246];
const EARPHONE_NAME: &str = "Generic Xiaomi Type-C Earphones 1";

/// A uinput stand-in with the reference earphones' name and capabilities.
struct Source {
    file: File,
    sysname: String,
}

impl Source {
    fn create() -> Self {
        let file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")
            .unwrap();
        let fd = file.as_raw_fd();
        // SAFETY: Each request takes the integer or struct it encodes, valid for the call.
        unsafe {
            assert_eq!(libc::ioctl(fd, UI_SET_EVBIT as _, 1), 0);
            for key in EARPHONE_KEYS {
                assert_eq!(
                    libc::ioctl(fd, UI_SET_KEYBIT as _, libc::c_int::from(key)),
                    0
                );
            }
            assert_eq!(libc::ioctl(fd, UI_SET_EVBIT as _, 4), 0);
            assert_eq!(libc::ioctl(fd, UI_SET_MSCBIT as _, 4), 0);
            let mut setup = UinputSetup::new(INPUT_ID);
            setup.name = [0; 80];
            setup.name[..EARPHONE_NAME.len()].copy_from_slice(EARPHONE_NAME.as_bytes());
            assert_eq!(libc::ioctl(fd, UI_DEV_SETUP as _, &setup), 0);
            assert_eq!(libc::ioctl(fd, UI_DEV_CREATE as _), 0);
            let mut name = [0u8; 64];
            assert!(libc::ioctl(fd, ui_get_sysname(64) as _, name.as_mut_ptr()) >= 0);
            let sysname =
                String::from_utf8(name.split(|&b| b == 0).next().unwrap().to_vec()).unwrap();
            Self { file, sysname }
        }
    }

    fn emit(&self, events: &[Event]) {
        (&self.file).write_all(&encode(events)).unwrap();
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        // SAFETY: UI_DEV_DESTROY takes no argument.
        unsafe { libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY as _) };
    }
}
const FAKE_EVENT: &str = "event90";

fn ev(kind: u16, code: u16, value: i32) -> Event {
    Event {
        timestamp_us: 0,
        kind,
        code,
        value,
    }
}

fn center(value: i32) -> Vec<Event> {
    vec![ev(4, 4, 0x000c_00cd), ev(1, 0xa4, value), ev(0, 0, 0)]
}

fn volume_up(value: i32) -> Vec<Event> {
    vec![ev(4, 4, 0x000c_00e9), ev(1, 0x73, value), ev(0, 0, 0)]
}

fn mknod(path: &Path, major: u32, minor: u32) {
    let _ = fs::remove_file(path);
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

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("{text}\n")).unwrap();
}

/// The real event node and minor number of a uinput device.
fn real_event(sysname: &str) -> (String, u32) {
    let dir = Path::new("/sys/devices/virtual/input").join(sysname);
    let event = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.unwrap().file_name().into_string().ok())
        .find(|name| name.starts_with("event"))
        .unwrap();
    let dev = fs::read_to_string(dir.join(&event).join("dev")).unwrap();
    (
        event,
        dev.trim().split_once(':').unwrap().1.parse().unwrap(),
    )
}

struct Fake(PathBuf);

impl Fake {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("earkey-run-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in [
            "sys/class/input",
            "sys/devices/virtual/input",
            "dev/input",
            "proc",
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        mknod(&root.join("dev/uinput"), 10, 223);
        Self(root)
    }

    /// Describes the reference USB earphones whose event node is the given real minor.
    fn earphones(&self, minor: u32) {
        let device = self.0.join("sys/devices/platform/soc/1-1");
        let interface = device.join("1-1:1.3");
        let input = interface.join("0003:0020:0B21.0001/input/input10");
        write(&input.join("name"), EARPHONE_NAME);
        for (field, value) in [
            ("bustype", "0003"),
            ("vendor", "0020"),
            ("product", "0b21"),
            ("version", "0201"),
        ] {
            write(&input.join("id").join(field), value);
        }
        write(&input.join("capabilities/ev"), "13");
        write(
            &input.join("capabilities/key"),
            "40 0 0 0 0 0 0 3800000800 e000000000000 0",
        );
        write(&input.join("capabilities/msc"), "10");
        write(&input.join("properties"), "0");
        write(&input.join(FAKE_EVENT).join("dev"), &format!("13:{minor}"));
        let class = self.0.join("sys/class/input").join(FAKE_EVENT);
        if fs::symlink_metadata(&class).is_err() {
            symlink(input.join(FAKE_EVENT), class).unwrap();
        }
        for (field, value) in [("bInterfaceNumber", "03"), ("bInterfaceClass", "03")] {
            write(&interface.join(field), value);
        }
        for (field, value) in [
            ("idVendor", "0020"),
            ("idProduct", "0b21"),
            ("bcdDevice", "0100"),
            ("serial", SERIAL),
        ] {
            write(&device.join(field), value);
        }
        let driver = self.0.join("sys/bus/usb/drivers/usbhid");
        fs::create_dir_all(&driver).unwrap();
        let _ = symlink(driver, interface.join("driver"));
        mknod(&self.0.join("dev/input").join(FAKE_EVENT), 13, minor);
    }

    /// Mirrors every live proxy except the stand-in source, as Android would open it.
    fn android(&self, source: &str) {
        let Ok(entries) = fs::read_dir("/sys/devices/virtual/input") else {
            return;
        };
        for entry in entries.flatten() {
            let sysname = entry.file_name().into_string().unwrap();
            if sysname == source
                || fs::read_to_string(entry.path().join("name"))
                    .map(|n| n.trim() != PROXY_NAME)
                    .unwrap_or(true)
            {
                continue;
            }
            let Some(event) = fs::read_dir(entry.path()).ok().and_then(|d| {
                d.flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .find(|n| n.starts_with("event"))
            }) else {
                continue;
            };
            let Ok(dev) = fs::read_to_string(entry.path().join(&event).join("dev")) else {
                continue;
            };
            let mirror = self
                .0
                .join("sys/devices/virtual/input")
                .join(&sysname)
                .join(&event);
            if mirror.exists() {
                continue;
            }
            let minor: u32 = dev.trim().split_once(':').unwrap().1.parse().unwrap();
            let node = self.0.join("dev/input").join(&event);
            mknod(&node, 13, minor);
            write(&mirror.join("dev"), dev.trim());
            let process = self.0.join("proc/4242");
            fs::create_dir_all(process.join("fd")).unwrap();
            fs::write(process.join("cmdline"), b"system_server\0").unwrap();
            let _ = symlink(&node, process.join("fd").join(&sysname));
            *PROXY_NODE.lock().unwrap() = Some(node);
        }
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

static PROXY_NODE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Stops the worker and the helper even when an assertion panics inside the scope.
struct Release<'a>(&'a AtomicBool, &'a AtomicBool);

impl Drop for Release<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        self.1.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Shared {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }

    fn wait_for(&self, needle: &str) {
        let end = Instant::now() + Duration::from_secs(10);
        while !self.text().contains(needle) {
            assert!(
                Instant::now() < end,
                "timed out waiting for {needle:?}:\n{}",
                self.text()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }
}

fn read_keys(file: &mut File, wait: Duration) -> Vec<(u16, i32)> {
    let size = std::mem::size_of::<libc::input_event>();
    let end = Instant::now() + wait;
    let mut keys = Vec::new();
    let mut buffer = vec![0u8; size * 64];
    while Instant::now() < end {
        match file.read(&mut buffer) {
            Ok(read) => {
                for chunk in buffer[..read].chunks_exact(size) {
                    // SAFETY: Each chunk holds one input_event.
                    let e: libc::input_event =
                        unsafe { std::ptr::read_unaligned(chunk.as_ptr().cast()) };
                    if e.type_ == 1 {
                        keys.push((e.code, e.value));
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5))
            }
            Err(error) => panic!("{error}"),
        }
    }
    keys
}

fn proxy_reader(report: &Shared, generation: u64) -> File {
    report.wait_for(&format!("worker_state=active:generation={generation}"));
    let node = PROXY_NODE.lock().unwrap().clone().unwrap();
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(node)
        .unwrap()
}

fn environment() -> Observed {
    parse(
        |name| {
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
        },
        "Linux version 4.14.357-reforge-g9b21e3df6c0a (b@h) (c) #2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026",
    )
    .unwrap()
}

#[test]
#[ignore = "needs /dev/uinput, CAP_MKNOD and input device access; see docs/build.md"]
fn the_policy_loop_blocks_reloads_survives_unplug_and_stops() {
    let store_dir = std::env::temp_dir().join(format!("earkey-run-store-{}", std::process::id()));
    let _ = fs::remove_dir_all(&store_dir);
    DirBuilder::new().mode(0o700).create(&store_dir).unwrap();
    let store = Store::open_dir(&store_dir).unwrap();
    let evidence = Evidence::XiaomiPixelosReforge20261110;
    store
        .enroll(&VerifiedEnrollment::from_reviewed_measurement(evidence, SERIAL).unwrap())
        .unwrap();
    apply(&store, &Selection::parse("center").unwrap(), true).unwrap();

    let fake = Fake::new();
    let mut source = Source::create();
    fake.earphones(real_event(&source.sysname).1);
    let source_name = Arc::new(Mutex::new(source.sysname.clone()));
    let environment = environment();
    let ctx = Context {
        root: &fake.0,
        environment: &environment,
        layout: LAYOUT,
        store: &store,
        poll: Duration::from_millis(50),
        readiness: Duration::from_secs(3),
        forward: Limits {
            stall: Duration::from_millis(100),
            session: Duration::from_secs(60),
            tick: Duration::from_millis(20),
        },
    };
    let stop = AtomicBool::new(false);
    let done = AtomicBool::new(false);
    let (frames, report) = (Shared::default(), Shared::default());
    let android = AtomicBool::new(false);

    let passed = thread::scope(|scope| {
        scope.spawn(|| {
            while !done.load(Ordering::SeqCst) {
                if android.load(Ordering::SeqCst) {
                    let name = source_name.lock().unwrap().clone();
                    fake.android(&name);
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        let worker = scope.spawn(|| run(&ctx, &stop, &mut frames.clone(), &mut report.clone()));
        let _release = Release(&stop, &done);

        thread::sleep(Duration::from_millis(400));
        let early = frames.0.lock().unwrap().len();
        thread::sleep(Duration::from_millis(1200));
        assert!(
            frames.0.lock().unwrap().len() >= early + 9 * 10,
            "progress stalled while Android had not opened the proxy"
        );
        assert!(!report.text().contains("proxy_ready"), "{}", report.text());
        android.store(true, Ordering::SeqCst);

        let mut output = proxy_reader(&report, 1);
        assert!(
            report
                .text()
                .contains("worker_state=active:generation=1:blocked=Center"),
            "{}",
            report.text()
        );
        thread::sleep(Duration::from_millis(100));
        source.emit(&center(1));
        source.emit(&center(0));
        source.emit(&volume_up(1));
        source.emit(&volume_up(0));
        assert_eq!(
            read_keys(&mut output, Duration::from_millis(500)),
            [(0x73, 1), (0x73, 0)]
        );

        apply(&store, &Selection::BlockAll, true).unwrap();
        let mut output = proxy_reader(&report, 2);
        assert!(
            report
                .text()
                .contains("active:generation=2:blocked=Center,VolumeUp,VolumeDown")
        );
        thread::sleep(Duration::from_millis(100));
        source.emit(&volume_up(1));
        source.emit(&volume_up(0));
        assert!(read_keys(&mut output, Duration::from_millis(300)).is_empty());

        drop(source);
        report.wait_for("worker_state=disconnected");
        source = Source::create();
        *source_name.lock().unwrap() = source.sysname.clone();
        let before = report
            .text()
            .matches("worker_state=active:generation=2")
            .count();
        fake.earphones(real_event(&source.sysname).1);
        let end = Instant::now() + Duration::from_secs(10);
        while report
            .text()
            .matches("worker_state=active:generation=2")
            .count()
            == before
        {
            assert!(
                Instant::now() < end,
                "no reacquire after replug:\n{}",
                report.text()
            );
            thread::sleep(Duration::from_millis(20));
        }

        stop.store(true, Ordering::SeqCst);
        let passed = worker.join().unwrap();
        done.store(true, Ordering::SeqCst);
        passed
    });
    assert!(passed, "{}", report.text());
    assert!(
        report.text().ends_with("worker_state=stopped\n"),
        "{}",
        report.text()
    );
    assert_eq!(
        store
            .read_runtime(earkey_policy::store::Runtime::Worker)
            .unwrap()
            .as_deref(),
        Some("state=stopped\nactive_generation=0\n")
    );
    assert_eq!(
        earkey_worker::run::status_text("active:generation=2:blocked=Center"),
        "state=active:generation=2:blocked=Center\nactive_generation=2\n"
    );
    let bytes = frames.0.lock().unwrap();
    assert!(bytes.len() > 9 && bytes[0] == 1, "started frame first");
    drop(bytes);
    let _ = fs::remove_dir_all(&store_dir);
}
