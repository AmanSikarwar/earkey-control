use earkey_policy::identity::{
    AndroidKey, Candidate, Environment, Evidence, Group, HidUsage, LinuxKey, NodeInstance, UsbHid,
    VerifiedEnrollment,
};
use earkey_policy::policy::{MAX_POLICY_BYTES, MatchError, Policy, PolicyError};
use serde_json::{Value, json};

const SERIAL: &str = "SYNTHETIC-ENROLLMENT";
const EVIDENCE: Evidence = Evidence::XiaomiPixelos20261107;
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

fn earphones(inode: u64) -> Candidate<'static> {
    Candidate {
        instance: NodeInstance {
            filesystem_device: 1,
            inode,
            character_device: 200,
        },
        usb_hid: Some(UsbHid {
            vendor: 32,
            product: 2849,
            revision: 256,
            interface: 3,
            interface_class: 3,
            hid_bus: 3,
            hid_vendor: 32,
            hid_product: 2849,
        }),
        usb_serial: Some(SERIAL),
        usb_driver: Some("usbhid"),
        is_virtual: false,
        input_id: [3, 32, 2849, 513],
        name: "Generic Xiaomi Type-C Earphones 1",
        layout_sha256: "4d28d904e72ea3beb4d32d73437cf9e7f1ed297a436d51d53b1a248a5cd5f773",
        event_types: &[0, 1, 4],
        keys: &[
            LinuxKey(113),
            LinuxKey(114),
            LinuxKey(115),
            LinuxKey(139),
            LinuxKey(163),
            LinuxKey(164),
            LinuxKey(165),
            LinuxKey(582),
        ],
        misc: &[4],
        properties: &[],
    }
}

fn parse(bytes: &[u8]) -> Result<Policy, PolicyError> {
    let enrollment = VerifiedEnrollment::from_reviewed_measurement(EVIDENCE, SERIAL).unwrap();
    Policy::parse(bytes, &[enrollment])
}

fn with_edit(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value = serde_json::from_str(REQUEST).unwrap();
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn strict_schema_rejects_malformed_or_ambiguous_json() {
    let mutations = [
        REQUEST.replace(
            "\"enabled\": true,",
            "\"enabled\": true, \"enabled\": false,",
        ),
        REQUEST.replace("\"seconds\": 0", "\"seconds\": 0, \"seconds\": 5"),
        REQUEST.replace("\"enabled\": true,", "\"enabled\": true, \"enable\": true,"),
        REQUEST.replace("\"enabled\": true,", ""),
        REQUEST.replace("\"ignore\"", "\"activate\""),
        REQUEST.replace("\"evdev_uinput\"", "\"framework\""),
        REQUEST.replace(
            "{\"enabled\": false, \"seconds\": 0, \"max_bytes\": 0}",
            "[false, 0, 0]",
        ),
        format!("{REQUEST} true"),
        format!("{}0{}", "[".repeat(200), "]".repeat(200)),
    ];
    for bytes in mutations {
        assert_eq!(
            parse(bytes.as_bytes()).err(),
            Some(PolicyError::InvalidJson)
        );
    }
    assert_eq!(parse(&[0xff]).err(), Some(PolicyError::InvalidJson));
    assert_eq!(
        parse(&vec![b' '; MAX_POLICY_BYTES + 1]).err(),
        Some(PolicyError::Oversized)
    );
    assert_eq!(
        parse(&with_edit(|p| p["schema_version"] = json!(2))).err(),
        Some(PolicyError::UnsupportedSchema(2)),
    );
}

#[test]
fn policy_cannot_create_or_widen_an_enrollment() {
    assert_eq!(
        Policy::parse(REQUEST.as_bytes(), &[]).err(),
        Some(PolicyError::UnverifiedProfile)
    );
    assert_eq!(
        parse(&with_edit(
            |p| p["profiles"][0]["usb_serial"] = json!("OTHER-SERIAL")
        ))
        .err(),
        Some(PolicyError::UnverifiedProfile),
    );
    assert_eq!(
        parse(&with_edit(
            |p| p["profiles"] = json!([p["profiles"][0], p["profiles"][0]])
        ))
        .err(),
        Some(PolicyError::DuplicateProfile),
    );
    for selection in [
        json!(["assistant"]),
        json!(["call_mute"]),
        json!([164]),
        json!([786637]),
    ] {
        assert_eq!(
            parse(&with_edit(
                |p| p["profiles"][0]["blocked_groups"] = selection
            ))
            .err(),
            Some(PolicyError::InvalidJson),
        );
    }
    for edit in [
        with_edit(|p| p["preset"] = json!("block_all")),
        with_edit(|p| p["profiles"][0]["blocked_groups"] = json!(["center", "center"])),
    ] {
        assert_eq!(parse(&edit).err(), Some(PolicyError::InvalidGroups));
    }
    assert_eq!(
        parse(&with_edit(|p| p["profiles"][0]["enabled"] = json!(false))).err(),
        Some(PolicyError::NoEnabledProfile),
    );
    assert_eq!(
        parse(&with_edit(
            |p| p["diagnostics"] = json!({"enabled": true, "seconds": 121, "max_bytes": 4})
        ))
        .err(),
        Some(PolicyError::InvalidDiagnostics),
    );
}

#[test]
fn controls_distinguish_namespaces_and_disclose_shared_effects() {
    let policy = parse(REQUEST.as_bytes()).unwrap();
    let controls: Vec<_> = policy.blocked_controls(0).collect();
    assert_eq!(controls.len(), 1);
    let center = controls[0];
    assert_eq!(center.group, Group::Center);
    assert_eq!(center.linux, LinuxKey(164));
    assert_eq!(
        center.hid,
        HidUsage {
            page: 12,
            usage: 205
        }
    );
    assert_eq!(center.android, AndroidKey::MediaPlayPause);
    assert!(
        center
            .observed_effects
            .contains(&"Open assistant and wake screen")
    );
    assert!(
        center
            .observed_effects
            .contains(&"Answer or end cellular call")
    );
    assert!(
        center
            .reported_effects
            .contains(&"Mute or unmute an ongoing call")
    );
    assert!(center.reported_effects.contains(&"Reject an incoming call"));

    let all = parse(&with_edit(|p| {
        p["preset"] = json!("block_all");
        p["profiles"][0]["blocked_groups"] = json!([]);
    }))
    .unwrap();
    assert_eq!(
        all.blocked_controls(0)
            .map(|control| control.linux)
            .collect::<Vec<_>>(),
        [LinuxKey(164), LinuxKey(115), LinuxKey(114)],
    );
    assert!(
        all.blocked_controls(0)
            .filter(|control| control.group != Group::Center)
            .all(|control| control
                .reported_effects
                .contains(&"Silence an incoming ringtone"))
    );
}

#[test]
fn exact_collection_required_and_other_input_untouched() {
    let policy = parse(REQUEST.as_bytes()).unwrap();
    let keyboard_with_media = Candidate {
        input_id: [3, 1, 1, 1],
        keys: &[LinuxKey(30), LinuxKey(164)],
        ..earphones(11)
    };
    let own_proxy = Candidate {
        is_virtual: true,
        ..earphones(12)
    };
    let unknown_serial = Candidate {
        usb_serial: Some("UNKNOWN"),
        ..earphones(13)
    };
    let no_serial = Candidate {
        usb_serial: None,
        ..earphones(14)
    };
    let wrong_ancestry = Candidate {
        usb_driver: Some("unknown"),
        ..earphones(15)
    };
    let absent_parent = Candidate {
        usb_hid: None,
        ..earphones(16)
    };
    let inventory = [
        keyboard_with_media,
        own_proxy,
        unknown_serial,
        no_serial,
        wrong_ancestry,
        absent_parent,
    ];
    assert!(policy.targets(&ENVIRONMENT, &inventory).unwrap().is_empty());

    let mixed_essential = Candidate {
        event_types: &[0, 1, 4, 5],
        ..earphones(17)
    };
    let keyboard_clone = Candidate {
        keys: &[LinuxKey(30), LinuxKey(164)],
        ..earphones(18)
    };
    let altered_layout = Candidate {
        layout_sha256: "",
        ..earphones(19)
    };
    let touch_properties = Candidate {
        properties: &[1],
        ..earphones(20)
    };
    for device in [
        mixed_essential,
        keyboard_clone,
        altered_layout,
        touch_properties,
    ] {
        assert_eq!(
            policy.targets(&ENVIRONMENT, &[device]).err(),
            Some(MatchError::ChangedCapabilities)
        );
    }
    assert_eq!(
        policy
            .targets(&ENVIRONMENT, &[earphones(1), earphones(2)])
            .err(),
        Some(MatchError::Ambiguous),
    );
}

#[test]
fn revalidation_requires_live_instance_and_unique_fresh_inventory() {
    let policy = parse(REQUEST.as_bytes()).unwrap();
    let mut targets = policy.targets(&ENVIRONMENT, &[earphones(1)]).unwrap();
    assert_eq!(targets.len(), 1);
    let target = targets.pop().unwrap();
    assert_eq!(target.instance().inode, 1);
    assert!(
        policy
            .revalidate(&target, &earphones(1), &ENVIRONMENT, &[earphones(1)])
            .unwrap()
    );
    let opened_wrong_serial = Candidate {
        usb_serial: Some("REPLACEMENT"),
        ..earphones(1)
    };
    let opened_with_switch = Candidate {
        event_types: &[0, 1, 4, 5],
        ..earphones(1)
    };
    for opened in [opened_wrong_serial, opened_with_switch] {
        assert!(
            !policy
                .revalidate(&target, &opened, &ENVIRONMENT, &[earphones(1)])
                .unwrap()
        );
    }
    assert!(
        !policy
            .revalidate(&target, &earphones(2), &ENVIRONMENT, &[earphones(2)])
            .unwrap()
    );
    assert!(
        !policy
            .revalidate(&target, &earphones(1), &ENVIRONMENT, &[])
            .unwrap()
    );
    assert_eq!(
        policy.revalidate(
            &target,
            &earphones(1),
            &ENVIRONMENT,
            &[earphones(1), earphones(2)]
        ),
        Err(MatchError::Ambiguous),
    );
    let new_target = policy
        .targets(&ENVIRONMENT, &[earphones(2)])
        .unwrap()
        .pop()
        .unwrap();
    assert!(
        policy
            .revalidate(&new_target, &earphones(2), &ENVIRONMENT, &[earphones(2)])
            .unwrap()
    );

    let changed_environment = Environment {
        api: 36,
        ..ENVIRONMENT
    };
    assert!(
        policy
            .targets(&changed_environment, &[earphones(1)])
            .unwrap()
            .is_empty()
    );
    let newer = parse(&with_edit(|p| p["generation"] = json!(1))).unwrap();
    assert!(
        !newer
            .revalidate(&target, &earphones(1), &ENVIRONMENT, &[earphones(1)])
            .unwrap()
    );
}

#[test]
fn inactive_policy_and_recovery_inhibit_targets() {
    assert!(
        Policy::inactive()
            .targets(&ENVIRONMENT, &[earphones(1)])
            .unwrap()
            .is_empty()
    );
    for (field, value) in [
        ("paused", json!(true)),
        ("boot_disabled", json!(true)),
        ("latch", json!("restart_limit")),
    ] {
        let policy = parse(&with_edit(|p| p["recovery"][field] = value)).unwrap();
        assert!(
            policy
                .targets(&ENVIRONMENT, &[earphones(1)])
                .unwrap()
                .is_empty()
        );
    }
    let no_groups = parse(&with_edit(|p| {
        p["profiles"][0]["blocked_groups"] = json!([])
    }))
    .unwrap();
    assert!(
        no_groups
            .targets(&ENVIRONMENT, &[earphones(1)])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn enrollment_from_observation_requires_the_reviewed_environment_identity_and_capabilities() {
    let device = earphones(7);
    let enrolled = VerifiedEnrollment::from_observation(EVIDENCE, &ENVIRONMENT, &device).unwrap();
    assert_eq!(enrolled.evidence(), EVIDENCE);
    assert_eq!(enrolled.serial(), SERIAL);
    assert!(Policy::parse(REQUEST.as_bytes(), &[enrolled]).is_ok());

    let other_kernel = Environment {
        kernel_release: "4.14.357-reforge-g9b21e3df6c0a",
        ..ENVIRONMENT
    };
    assert!(VerifiedEnrollment::from_observation(EVIDENCE, &other_kernel, &device).is_none());

    let rejected = [
        Candidate {
            usb_serial: None,
            ..earphones(7)
        },
        Candidate {
            usb_serial: Some(" "),
            ..earphones(7)
        },
        Candidate {
            is_virtual: true,
            ..earphones(7)
        },
        Candidate {
            usb_driver: Some("hid-generic"),
            ..earphones(7)
        },
        Candidate {
            name: "Generic Xiaomi Type-C Earphones 2",
            ..earphones(7)
        },
        Candidate {
            misc: &[],
            ..earphones(7)
        },
        Candidate {
            layout_sha256: "0000",
            ..earphones(7)
        },
    ];
    for candidate in &rejected {
        assert!(VerifiedEnrollment::from_observation(EVIDENCE, &ENVIRONMENT, candidate).is_none());
    }
}
