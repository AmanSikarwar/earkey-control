#![cfg(any(target_os = "linux", target_os = "android"))]
//! End-to-end forwarding between two real uinput devices in the test kernel.
//! Run in the documented container with `--device /dev/uinput`; never on the phone.

use earkey_engine::Event;
use earkey_policy::identity::{Evidence, LinuxKey, NodeInstance};
use earkey_worker::device::Opened;
use earkey_worker::forward::{End, Limits, emit, run};
use earkey_worker::inventory::Observation;
use earkey_worker::proxy::Proxy;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const INPUT_ID: [u16; 4] = [3, 0x20, 0x0b21, 0x0201];
const KEYS: [LinuxKey; 3] = [LinuxKey(0x72), LinuxKey(0x73), LinuxKey(0xa4)];
const EVIDENCE: Evidence = Evidence::XiaomiPixelosReforge20261110;
const CENTER_SCAN: i32 = 0x000c_00cd;
const LIMITS: Limits = Limits {
    stall: Duration::from_millis(100),
    session: Duration::from_secs(5),
    tick: Duration::from_millis(20),
};
static SERIAL: Mutex<()> = Mutex::new(());

fn ev(kind: u16, code: u16, value: i32) -> Event {
    Event {
        timestamp_us: 0,
        kind,
        code,
        value,
    }
}

fn press(value: i32) -> Vec<Event> {
    vec![ev(4, 4, CENTER_SCAN), ev(1, 0xa4, value), ev(0, 0, 0)]
}

struct Dev(PathBuf);

impl Dev {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "earkey-forward-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("input")).unwrap();
        Self(root)
    }

    /// Creates the /dev node for a proxy's event device and returns its name and path.
    fn node(&self, proxy: &Proxy) -> (String, PathBuf) {
        let dir = Path::new("/sys/devices/virtual/input").join(&proxy.sysname);
        let event = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.unwrap().file_name().into_string().ok())
            .find(|name| name.starts_with("event"))
            .unwrap();
        let dev = fs::read_to_string(dir.join(&event).join("dev")).unwrap();
        let (major, minor) = dev.trim().split_once(':').unwrap();
        let path = self.0.join("input").join(&event);
        let c = CString::new(path.as_os_str().as_bytes()).unwrap();
        let device = libc::makedev(major.parse().unwrap(), minor.parse().unwrap());
        // SAFETY: The path is NUL-terminated and valid for the call.
        assert_eq!(
            unsafe { libc::mknod(c.as_ptr(), libc::S_IFCHR | 0o600, device) },
            0
        );
        (event, path)
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn observation(event: &str, path: &Path) -> Observation {
    let metadata = fs::symlink_metadata(path).unwrap();
    Observation {
        event: event.into(),
        instance: NodeInstance {
            filesystem_device: metadata.dev(),
            inode: metadata.ino(),
            character_device: metadata.rdev(),
        },
        usb_hid: None,
        usb_serial: None,
        usb_driver: None,
        is_virtual: true,
        input_id: INPUT_ID,
        name: String::new(),
        event_types: vec![],
        keys: vec![],
        misc: vec![],
        properties: vec![],
    }
}

fn reader(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .unwrap()
}

/// Reads (type, code, value) records until `count` arrive or the deadline passes.
fn read_records(file: &mut File, count: usize, wait: Duration) -> Vec<(u16, u16, i32)> {
    let size = std::mem::size_of::<libc::input_event>();
    let end = Instant::now() + wait;
    let mut records = Vec::new();
    let mut buffer = vec![0u8; size * 64];
    while records.len() < count && Instant::now() < end {
        match file.read(&mut buffer) {
            Ok(read) => {
                for chunk in buffer[..read].chunks_exact(size) {
                    // SAFETY: Each chunk holds one input_event.
                    let e: libc::input_event =
                        unsafe { std::ptr::read_unaligned(chunk.as_ptr().cast()) };
                    records.push((e.type_, e.code, e.value));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5))
            }
            Err(error) => panic!("{error}"),
        }
    }
    records
}

#[test]
#[ignore = "needs /dev/uinput and CAP_MKNOD; see docs/build.md"]
fn passthrough_forwards_exactly_and_grab_hides_the_source() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dev = Dev::new();
    let source = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let proxy = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let (source_event, source_path) = dev.node(&source);
    let (_, proxy_path) = dev.node(&proxy);
    let mut bystander = reader(&source_path);
    let mut output = reader(&proxy_path);
    let opened = Opened::open(&dev.0, &observation(&source_event, &source_path)).unwrap();
    let stop = AtomicBool::new(false);
    let counter = AtomicU64::new(0);

    let end = thread::scope(|scope| {
        let session = scope.spawn(|| {
            run(&opened, &proxy, EVIDENCE, &[], LIMITS, &stop, |c| {
                counter.store(c, Ordering::SeqCst)
            })
        });
        thread::sleep(Duration::from_millis(200));
        emit(&source, &press(1)).unwrap();
        emit(&source, &press(0)).unwrap();
        let forwarded = read_records(&mut output, 6, Duration::from_secs(2));
        assert_eq!(
            forwarded,
            [
                (4, 4, CENTER_SCAN),
                (1, 0xa4, 1),
                (0, 0, 0),
                (4, 4, CENTER_SCAN),
                (1, 0xa4, 0),
                (0, 0, 0)
            ]
        );
        assert!(read_records(&mut bystander, 1, Duration::from_millis(100)).is_empty());
        let before = counter.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(100));
        assert!(
            counter.load(Ordering::SeqCst) > before,
            "idle loop must report progress"
        );
        stop.store(true, Ordering::SeqCst);
        session.join().unwrap()
    });
    assert_eq!(end.unwrap(), End::Stopped);

    emit(&source, &press(1)).unwrap();
    emit(&source, &press(0)).unwrap();
    assert_eq!(
        read_records(&mut bystander, 6, Duration::from_secs(1)).len(),
        6,
        "grab must be released"
    );
}

#[test]
#[ignore = "needs /dev/uinput and CAP_MKNOD; see docs/build.md"]
fn stop_during_a_held_key_releases_it_on_the_proxy() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dev = Dev::new();
    let source = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let proxy = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let (source_event, source_path) = dev.node(&source);
    let (_, proxy_path) = dev.node(&proxy);
    let mut output = reader(&proxy_path);
    let opened = Opened::open(&dev.0, &observation(&source_event, &source_path)).unwrap();
    let stop = AtomicBool::new(false);

    let end = thread::scope(|scope| {
        let session = scope.spawn(|| run(&opened, &proxy, EVIDENCE, &[], LIMITS, &stop, |_| {}));
        thread::sleep(Duration::from_millis(200));
        emit(&source, &press(1)).unwrap();
        assert_eq!(
            read_records(&mut output, 3, Duration::from_secs(2)).len(),
            3
        );
        stop.store(true, Ordering::SeqCst);
        session.join().unwrap()
    });
    assert_eq!(end.unwrap(), End::Stopped);
    assert_eq!(
        read_records(&mut output, 2, Duration::from_secs(1)),
        [(1, 0xa4, 0), (0, 0, 0)]
    );
    emit(&source, &press(0)).unwrap();
}

#[test]
#[ignore = "needs /dev/uinput and CAP_MKNOD; see docs/build.md"]
fn a_key_held_at_grab_time_abandons_the_session() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dev = Dev::new();
    let source = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let proxy = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let (source_event, source_path) = dev.node(&source);
    let (_, proxy_path) = dev.node(&proxy);
    let mut output = reader(&proxy_path);
    let opened = Opened::open(&dev.0, &observation(&source_event, &source_path)).unwrap();
    emit(&source, &press(1)).unwrap();
    let stop = AtomicBool::new(false);
    assert_eq!(
        run(&opened, &proxy, EVIDENCE, &[], LIMITS, &stop, |_| {}).unwrap(),
        End::NotNeutral
    );
    assert!(read_records(&mut output, 1, Duration::from_millis(100)).is_empty());

    emit(&source, &press(0)).unwrap();
    let mut bystander = reader(&source_path);
    emit(&source, &press(1)).unwrap();
    emit(&source, &press(0)).unwrap();
    assert_eq!(
        read_records(&mut bystander, 6, Duration::from_secs(1)).len(),
        6,
        "grab must be released"
    );
}

#[test]
#[ignore = "needs /dev/uinput and CAP_MKNOD; see docs/build.md"]
fn a_removed_source_reports_its_fault_after_releasing_held_keys() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dev = Dev::new();
    let source = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let proxy = Proxy::create(Path::new("/dev"), INPUT_ID, &KEYS).unwrap();
    let (source_event, source_path) = dev.node(&source);
    let (_, proxy_path) = dev.node(&proxy);
    let mut output = reader(&proxy_path);
    let opened = Opened::open(&dev.0, &observation(&source_event, &source_path)).unwrap();
    let stop = AtomicBool::new(false);
    let end = thread::scope(|scope| {
        let session = scope.spawn(|| run(&opened, &proxy, EVIDENCE, &[], LIMITS, &stop, |_| {}));
        thread::sleep(Duration::from_millis(200));
        emit(&source, &press(1)).unwrap();
        assert_eq!(
            read_records(&mut output, 3, Duration::from_secs(2)).len(),
            3
        );
        drop(source);
        session.join().unwrap()
    });
    assert!(matches!(end.unwrap(), End::Fault(_)));
    // The kernel releases the removed device's held keys, marking the report with value 1;
    // the engine forwards that release before the read fails.
    let released = read_records(&mut output, 2, Duration::from_secs(1));
    assert_eq!(released[0], (1, 0xa4, 0));
    assert!(matches!(released[1], (0, 0, 0 | 1)), "{released:?}");
}
