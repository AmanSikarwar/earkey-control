//! Read-only acquisition check for one enrolled accessory.
//!
//! It runs environment matching, inventory, enrollment from the live
//! observation, layout-override detection, targeting, open, revalidation and
//! the neutral-state query. It never reads events, grabs or writes anything.
//! The report carries only counts and outcomes, never serials or other names.

use crate::device::Opened;
use crate::environment::Observed;
use crate::inventory::{self, Observation};
use earkey_policy::identity::{Evidence, VerifiedEnrollment};
use earkey_policy::policy::Policy;
use std::path::Path;

/// Evidence the check may select, by environment.
pub const EVIDENCE: [Evidence; 2] = [
    Evidence::XiaomiPixelos20261107,
    Evidence::XiaomiPixelosReforge20261110,
];
const LAYOUT_DIRS: [&str; 4] = [
    "odm/usr/keylayout",
    "vendor/usr/keylayout",
    "system/usr/keylayout",
    "data/system/devices/keylayout",
];
const IDC_DIRS: [&str; 4] = [
    "odm/usr/idc",
    "vendor/usr/idc",
    "system/usr/idc",
    "data/system/devices/idc",
];

/// Ordered `key=value` lines and the overall result.
#[derive(Debug, Default)]
pub struct Report {
    pub lines: Vec<(&'static str, String)>,
    pub passed: bool,
}

impl Report {
    fn note(&mut self, key: &'static str, value: impl ToString) {
        self.lines.push((key, value.to_string()));
    }

    fn fail(mut self, key: &'static str, value: impl ToString) -> Self {
        self.note(key, value);
        self.note("check", "fail");
        self
    }
}

/// Runs the check under `root` (`/` on the phone) with the digest of the layout Android selects.
pub fn run(root: &Path, environment: &Observed, layout_sha256: &str) -> Report {
    let mut report = Report::default();
    if layout_sha256.len() != 64
        || !layout_sha256
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return report.fail("layout", "invalid_digest");
    }
    let env = environment.as_environment();
    let Some(evidence) = EVIDENCE.into_iter().find(|e| e.matches_environment(&env)) else {
        return report.fail("evidence", "none");
    };
    report.note(
        "evidence",
        serde_json::to_value(evidence)
            .unwrap_or_default()
            .as_str()
            .unwrap_or("?"),
    );

    let (sys, dev) = (root.join("sys"), root.join("dev"));
    let inventory = match inventory::scan(&sys, &dev) {
        Ok(inventory) => inventory,
        Err(error) => return report.fail("inventory", format!("error:{:?}", error.kind())),
    };
    report.note("inventory_nodes", inventory.len());

    let enrolled: Vec<(&Observation, VerifiedEnrollment<'_>)> = inventory
        .iter()
        .filter_map(|observed| {
            VerifiedEnrollment::from_observation(evidence, &env, &observed.candidate(layout_sha256))
                .map(|enrollment| (observed, enrollment))
        })
        .collect();
    let [(observed, enrollment)] = enrolled.as_slice() else {
        return report.fail(
            "enrollment",
            if enrolled.is_empty() {
                "none"
            } else {
                "ambiguous"
            },
        );
    };
    report.note("enrollment", "observed");

    if let Some(kind) = layout_override(root, observed) {
        return report.fail("layout", format!("override_present:{kind}"));
    }
    report.note("layout", "generic");

    let request = serde_json::json!({
        "schema_version": 1,
        "generation": 0,
        "enabled": true,
        "preset": "block_all",
        "profiles": [{
            "evidence": evidence,
            "usb_serial": enrollment.serial(),
            "enabled": true,
            "blocked_groups": [],
        }],
        "requested_backend": "evdev_uinput",
        "unknown_accessory": "ignore",
        "diagnostics": {"enabled": false, "seconds": 0, "max_bytes": 0},
        "recovery": {"paused": false, "boot_disabled": false, "latch": "clear"},
    });
    let policy = match Policy::parse(
        request.to_string().as_bytes(),
        std::slice::from_ref(enrollment),
    ) {
        Ok(policy) => policy,
        Err(error) => return report.fail("policy", format!("{error:?}")),
    };
    let candidates: Vec<_> = inventory
        .iter()
        .map(|o| o.candidate(layout_sha256))
        .collect();
    let targets = match policy.targets(&env, &candidates) {
        Ok(targets) => targets,
        Err(error) => return report.fail("targets", format!("{error:?}")),
    };
    let [target] = targets.as_slice() else {
        return report.fail("targets", targets.len());
    };
    if target.instance() != observed.instance {
        return report.fail("targets", "other_instance");
    }
    report.note("targets", 1);

    let opened = match Opened::open(&dev, observed) {
        Ok(opened) => opened,
        Err(error) => return report.fail("open", format!("error:{:?}", error.kind())),
    };
    report.note("open", "ok");

    let fresh = match inventory::scan(&sys, &dev) {
        Ok(fresh) => fresh,
        Err(error) => return report.fail("rescan", format!("error:{:?}", error.kind())),
    };
    let fresh: Vec<_> = fresh.iter().map(|o| o.candidate(layout_sha256)).collect();
    match policy.revalidate(
        target,
        &opened.candidate(observed, layout_sha256),
        &env,
        &fresh,
    ) {
        Ok(true) => report.note("revalidate", "ok"),
        Ok(false) => return report.fail("revalidate", "mismatch"),
        Err(error) => return report.fail("revalidate", format!("{error:?}")),
    }

    match opened.pressed_keys() {
        Ok(keys) if keys.is_empty() => report.note("neutral", "yes"),
        Ok(keys) => return report.fail("neutral", format!("no:{}", keys.len())),
        Err(error) => return report.fail("neutral", format!("error:{:?}", error.kind())),
    }
    report.note("check", "pass");
    report.passed = true;
    report
}

/// A device-specific key layout or input configuration would replace `Generic.kl`.
fn layout_override(root: &Path, observed: &Observation) -> Option<&'static str> {
    let [_, vendor, product, version] = observed.input_id;
    let canonical: String = observed
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stems = [
        format!("Vendor_{vendor:04x}_Product_{product:04x}_Version_{version:04x}"),
        format!("Vendor_{vendor:04x}_Product_{product:04x}"),
        canonical,
    ];
    for (dirs, extension, kind) in [(LAYOUT_DIRS, "kl", "kl"), (IDC_DIRS, "idc", "idc")] {
        for dir in dirs {
            for stem in &stems {
                if root
                    .join(dir)
                    .join(format!("{stem}.{extension}"))
                    .symlink_metadata()
                    .is_ok()
                {
                    return Some(kind);
                }
            }
        }
    }
    None
}
