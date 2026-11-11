use earkey_policy::identity::Evidence;
use earkey_worker::environment::{PROPERTIES, evidence_for, parse};
use std::collections::HashMap;

const REFORGE_VERSION: &str = "Linux version 4.14.357-reforge-g9b21e3df6c0a (Manipulator@LineageOS) (Neutron clang version 24.0.0, Neutron LLD 24.0.0) #2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026\n";

fn properties() -> HashMap<&'static str, String> {
    [
        ("ro.product.model", "M2101K6P"),
        ("ro.product.device", "sweetin"),
        ("ro.build.version.sdk", "37"),
        ("ro.custom.version", "PixelOS_sweet-17.0-20261030-2116"),
        ("ro.build.id", "CP2A.260605.016"),
        ("ro.build.version.incremental", "1793394994"),
    ]
    .into_iter()
    .map(|(name, value)| (name, value.to_owned()))
    .collect()
}

fn read(
    props: &HashMap<&'static str, String>,
    version: &str,
) -> std::io::Result<earkey_worker::environment::Observed> {
    parse(|name| props.get(name).cloned(), version)
}

#[test]
fn reference_phone_values_match_only_the_reforge_evidence() {
    let observed = read(&properties(), REFORGE_VERSION).unwrap();
    assert_eq!(observed.kernel_release, "4.14.357-reforge-g9b21e3df6c0a");
    assert_eq!(
        observed.kernel_build,
        "#2 SMP PREEMPT Mon Nov 9 19:36:49 WIB 2026"
    );
    assert_eq!(observed.api, 37);
    let environment = observed.as_environment();
    assert!(Evidence::XiaomiPixelosReforge20261110.matches_environment(&environment));
    assert!(!Evidence::XiaomiPixelos20261107.matches_environment(&environment));

    let stage1 = "Linux version 4.14.357-openela-VantomKernel (builder@host) (clang version 1 (https://example.invalid) #x) #1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026";
    let observed = read(&properties(), stage1).unwrap();
    assert_eq!(
        observed.kernel_build,
        "#1 SMP PREEMPT Sun Oct 11 07:45:56 UTC 2026"
    );
    assert!(Evidence::XiaomiPixelos20261107.matches_environment(&observed.as_environment()));
}

#[test]
fn updated_reforge_is_outside_the_production_evidence() {
    let version = "Linux version 4.14.357-reforge-gc216b1fe2466 (b) (c) #2 SMP PREEMPT Tue Nov 10 05:21:32 UTC 2026";
    let observed = read(&properties(), version).unwrap();
    assert_eq!(evidence_for(&observed), None);
    #[cfg(feature = "reforge-passthrough")]
    {
        let evidence = Evidence::XiaomiPixelosReforge20261111;
        assert!(evidence.matches_environment(&observed.as_environment()));
        let mut drifted = observed;
        drifted.kernel_build.push('1');
        assert!(!evidence.matches_environment(&drifted.as_environment()));
        let mut drifted = read(&properties(), version).unwrap();
        drifted.incremental.push('1');
        assert!(!evidence.matches_environment(&drifted.as_environment()));
        let earlier = read(&properties(), REFORGE_VERSION).unwrap();
        assert!(!evidence.matches_environment(&earlier.as_environment()));
    }
}

#[test]
fn missing_empty_truncated_or_malformed_properties_fail() {
    for name in PROPERTIES.into_iter().chain(["ro.build.version.sdk"]) {
        let mut props = properties();
        props.remove(name);
        assert!(read(&props, REFORGE_VERSION).is_err(), "missing {name}");
        for bad in ["", "a\nb", "\u{7}", &"x".repeat(91)] {
            let mut props = properties();
            props.insert(name, bad.to_owned());
            assert!(read(&props, REFORGE_VERSION).is_err(), "{name}={bad:?}");
        }
    }
    let mut props = properties();
    props.insert("ro.build.version.sdk", "thirty".into());
    assert!(read(&props, REFORGE_VERSION).is_err());
    props.insert("ro.custom.version", "x".repeat(90));
    props.insert("ro.build.version.sdk", "37".into());
    assert_eq!(read(&props, REFORGE_VERSION).unwrap().rom.len(), 90);
}

#[test]
fn malformed_proc_version_fails() {
    for bad in [
        "",
        "Linux 4.14 (b) (c) #1 SMP",
        "Linux version  (b) (c) #1 SMP",
        "Linux version 4.14.357 (b) (c) SMP PREEMPT",
        "Linux version 4.14.357 (b) (c) #",
        "Linux version 4.14.357 (b) (c) #1 SMP\u{7}",
        "Linux version 4.14.357 (b) (c) #1 SMP\nextra",
    ] {
        assert!(read(&properties(), bad).is_err(), "{bad:?}");
    }
}
