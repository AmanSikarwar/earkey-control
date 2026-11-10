use earkey_policy::identity::NodeInstance;
use earkey_worker::inventory::Observation;
use earkey_worker::layout::{generic_digest, has_override};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "earkey-layout-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("system/usr/keylayout")).unwrap();
        Self(root)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn observation() -> Observation {
    Observation {
        event: "event9".into(),
        instance: NodeInstance {
            filesystem_device: 0,
            inode: 0,
            character_device: 0,
        },
        usb_hid: None,
        usb_serial: None,
        usb_driver: None,
        is_virtual: false,
        input_id: [3, 0x20, 0x0b21, 0x0201],
        name: "Generic Xiaomi Type-C Earphones 1".into(),
        event_types: vec![],
        keys: vec![],
        misc: vec![],
        properties: vec![],
    }
}

#[test]
fn generic_digest_matches_published_sha256_vectors() {
    let root = Root::new();
    let generic = root.0.join("system/usr/keylayout/Generic.kl");
    for (bytes, expected) in [
        (
            &b""[..],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
    ] {
        fs::write(&generic, bytes).unwrap();
        assert_eq!(generic_digest(&root.0).unwrap(), expected);
    }
}

#[test]
fn missing_directory_or_oversized_layout_is_rejected() {
    let root = Root::new();
    let generic = root.0.join("system/usr/keylayout/Generic.kl");
    assert_eq!(
        generic_digest(&root.0).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    fs::create_dir(&generic).unwrap();
    assert_eq!(
        generic_digest(&root.0).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    fs::remove_dir(&generic).unwrap();
    fs::write(&generic, vec![b'#'; 64 * 1024]).unwrap();
    assert!(generic_digest(&root.0).is_ok());
    fs::write(&generic, vec![b'#'; 64 * 1024 + 1]).unwrap();
    assert_eq!(
        generic_digest(&root.0).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn device_specific_files_are_detected_by_id_and_name() {
    let root = Root::new();
    assert!(!has_override(&root.0, &observation()));
    for file in [
        "vendor/usr/keylayout/Vendor_0020_Product_0b21_Version_0201.kl",
        "odm/usr/keylayout/Vendor_0020_Product_0b21.kl",
        "data/system/devices/keylayout/Generic_Xiaomi_Type-C_Earphones_1.kl",
        "vendor/usr/idc/Generic_Xiaomi_Type-C_Earphones_1.idc",
    ] {
        let root = Root::new();
        let path = root.0.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"").unwrap();
        assert!(has_override(&root.0, &observation()), "{file}");
    }
    let root = Root::new();
    fs::create_dir_all(root.0.join("vendor/usr/keylayout")).unwrap();
    fs::write(
        root.0
            .join("vendor/usr/keylayout/Vendor_0020_Product_0b22.kl"),
        b"",
    )
    .unwrap();
    assert!(!has_override(&root.0, &observation()));
}
