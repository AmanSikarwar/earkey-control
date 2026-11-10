#![cfg(any(target_os = "linux", target_os = "android"))]

use earkey_worker::environment::parse;
use earkey_worker::proxy_check::{Classification, classify, run};
use std::path::Path;
use std::time::Duration;

const DUMPSYS: &str = "Event Hub State:
  BuiltInKeyboardId: -2
  Devices:
    -1: Virtual
      Classes: KEYBOARD | ALPHAKEY | VIRTUAL
      Path: <virtual>
    9: Generic Xiaomi Type-C Earphones 1
      Classes: KEYBOARD | EXTERNAL
      Path: /dev/input/event9
      Descriptor: 0123
      KeyLayoutFile: /system/usr/keylayout/Generic.kl
      KeyCharacterMapFile: /system/usr/keychars/Generic.kcm
      ConfigurationFile: <none>
    12: EarKey Control proxy
      Classes: KEYBOARD | EXTERNAL
      Path: /dev/input/event12
      Enabled: true
      Descriptor: 4567
      UniqueId: 
      KeyLayoutFile: /system/usr/keylayout/Generic.kl
      KeyCharacterMapFile: /system/usr/keychars/Generic.kcm
      ConfigurationFile: <none>
    13: uinput-fpc
      Classes: KEYBOARD
      Path: /dev/input/event13

Input Reader State (Nums of device: 3):
  Device 12: EarKey Control proxy
    Classes: KEYBOARD | EXTERNAL
";

#[test]
fn classify_reads_only_the_named_event_hub_block() {
    assert_eq!(
        classify(DUMPSYS, "EarKey Control proxy").unwrap(),
        Classification {
            classes: "KEYBOARD | EXTERNAL".into(),
            path: "/dev/input/event12".into(),
            key_layout: "/system/usr/keylayout/Generic.kl".into(),
            key_character_map: "/system/usr/keychars/Generic.kcm".into(),
            configuration: "<none>".into(),
        }
    );
    assert_eq!(classify(DUMPSYS, "uinput-fpc").unwrap().key_layout, "");
    assert_eq!(classify(DUMPSYS, "Virtual").unwrap().path, "<virtual>");
    assert!(classify(DUMPSYS, "EarKey Control").is_none());
    assert!(classify("", "EarKey Control proxy").is_none());
    assert!(classify("Device 12: EarKey Control proxy\n", "EarKey Control proxy").is_none());
}

#[test]
fn a_failed_check_never_creates_a_proxy() {
    let environment = parse(
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
    .unwrap();
    let report = run(
        Path::new("/nonexistent-earkey-root"),
        &environment,
        Duration::from_millis(10),
        || panic!("dumpsys must not run"),
    );
    assert!(!report.passed);
    assert!(report.checked.is_none());
    let lines: Vec<_> = report
        .lines
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    assert_eq!(
        lines,
        ["layout=unavailable:NotFound", "check=fail", "proxy=skipped"]
    );
}

mod exclusion {
    use earkey_policy::identity::{Environment, Evidence, LinuxKey, NodeInstance};
    use earkey_worker::check::Checked;
    use earkey_worker::inventory::Observation;
    use earkey_worker::proxy::PROXY_NAME;
    use earkey_worker::proxy_check::{excluded, proxy_keys};

    const LAYOUT: &str = "4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773";
    const ENVIRONMENT: Environment<'static> = Environment {
        model: "M2101K6P",
        device: "sweetin",
        api: 37,
        rom: "PixelOS_sweet-17.0-20261030-2116",
        build_id: "CP2A.260605.016",
        incremental: "1793394994",
        kernel_release: "4.14.357-reforge-g9b21e3df6c0a",
        kernel_build: "#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026",
    };
    const CHECKED: Checked = Checked {
        evidence: Evidence::XiaomiPixelosReforge20261110,
        input_id: [3, 0x20, 0x0b21, 0x0201],
    };

    /// The proxy as the inventory reports it: bitmap keys in ascending order.
    fn proxy() -> Observation {
        Observation {
            event: "event6".into(),
            instance: NodeInstance {
                filesystem_device: 1,
                inode: 2,
                character_device: 3,
            },
            usb_hid: None,
            usb_serial: None,
            usb_driver: None,
            is_virtual: true,
            input_id: CHECKED.input_id,
            name: PROXY_NAME.into(),
            event_types: vec![0, 1, 4],
            keys: vec![LinuxKey(0x72), LinuxKey(0x73), LinuxKey(0xa4)],
            misc: vec![4],
            properties: vec![],
        }
    }

    #[test]
    fn proxy_keys_are_ascending_although_the_evidence_lists_center_first() {
        let evidence: Vec<u16> = CHECKED
            .evidence
            .controls()
            .iter()
            .map(|c| c.linux.0)
            .collect();
        assert_eq!(evidence, [0xa4, 0x73, 0x72]);
        assert_eq!(
            proxy_keys(CHECKED.evidence),
            [LinuxKey(0x72), LinuxKey(0x73), LinuxKey(0xa4)]
        );
    }

    #[test]
    fn the_reported_proxy_is_excluded_and_any_deviation_is_not() {
        assert!(excluded(&proxy(), CHECKED, &ENVIRONMENT, LAYOUT));
        let deviations: [fn(&mut Observation); 7] = [
            |o| o.is_virtual = false,
            |o| o.name = "Generic Xiaomi Type-C Earphones 1".into(),
            |o| o.input_id[3] = 0x0202,
            |o| o.event_types = vec![0, 1, 4, 20],
            |o| o.keys.push(LinuxKey(0x71)),
            |o| o.misc = vec![],
            |o| o.keys.reverse(),
        ];
        for deviate in deviations {
            let mut observed = proxy();
            deviate(&mut observed);
            assert!(!excluded(&observed, CHECKED, &ENVIRONMENT, LAYOUT));
        }
    }
}
