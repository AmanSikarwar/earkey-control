//! Read-only acquisition check for one enrolled accessory.
//!
//! It runs layout hashing, environment matching, inventory, layout-override
//! detection, enrollment from the live observation, targeting, open, revalidation and
//! the neutral-state query. It never reads events, grabs or writes anything.
//! The report carries only counts and outcomes, never serials or other names.

use crate::device::Opened;
use crate::environment::{Observed, evidence_for};
use crate::inventory::{self, Observation};
use crate::layout;
use earkey_policy::identity::{Evidence, VerifiedEnrollment};
use earkey_policy::policy::Policy;
use std::path::Path;

/// Digest given to a node whose layout is overridden; it matches no evidence.
const OVERRIDDEN: &str = "override";

/// Ordered `key=value` lines and the overall result.
#[derive(Debug, Default)]
pub struct Report {
    pub lines: Vec<(&'static str, String)>,
    pub passed: bool,
    /// Set only when every step passed.
    pub checked: Option<Checked>,
}

/// What a passing check verified, for later acquisition steps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Checked {
    pub evidence: Evidence,
    /// Input ID read from the opened descriptor.
    pub input_id: [u16; 4],
}

impl Report {
    pub(crate) fn note(&mut self, key: &'static str, value: impl ToString) {
        self.lines.push((key, value.to_string()));
    }

    fn fail(mut self, key: &'static str, value: impl ToString) -> Self {
        self.note(key, value);
        self.note("check", "fail");
        self
    }
}

/// Runs the check under `root` (`/` on the phone), hashing the layout itself.
pub fn run(root: &Path, environment: &Observed) -> Report {
    match layout::generic_digest(root) {
        Ok(digest) => evaluate(root, environment, &digest),
        Err(error) => Report::default().fail("layout", format!("unavailable:{:?}", error.kind())),
    }
}

/// Runs every step after layout hashing with the given `Generic.kl` digest.
pub fn evaluate(root: &Path, environment: &Observed, layout_sha256: &str) -> Report {
    acquire_with(root, environment, layout_sha256).0
}

/// Like [`run`], but keeps the revalidated read-only descriptor when every step passed.
pub fn acquire(root: &Path, environment: &Observed) -> (Report, Option<Opened>) {
    match layout::generic_digest(root) {
        Ok(digest) => acquire_with(root, environment, &digest),
        Err(error) => (
            Report::default().fail("layout", format!("unavailable:{:?}", error.kind())),
            None,
        ),
    }
}

fn acquire_with(
    root: &Path,
    environment: &Observed,
    layout_sha256: &str,
) -> (Report, Option<Opened>) {
    let mut report = Report::default();
    report.note("layout_sha256", layout_sha256);
    let env = environment.as_environment();
    let Some(evidence) = evidence_for(environment) else {
        return (report.fail("evidence", "none"), None);
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
        Err(error) => {
            return (
                report.fail("inventory", format!("error:{:?}", error.kind())),
                None,
            );
        }
    };
    report.note("inventory_nodes", inventory.len());

    let digests = node_digests(root, &inventory, layout_sha256);
    let overrides = digests
        .iter()
        .filter(|&&digest| digest == OVERRIDDEN)
        .count();
    if overrides > 0 {
        report.note("layout_overrides", overrides);
    }
    let enrolled: Vec<(&Observation, VerifiedEnrollment<'_>)> = inventory
        .iter()
        .zip(&digests)
        .filter_map(|(observed, digest)| {
            VerifiedEnrollment::from_observation(evidence, &env, &observed.candidate(digest))
                .map(|enrollment| (observed, enrollment))
        })
        .collect();
    let [(observed, enrollment)] = enrolled.as_slice() else {
        return (
            report.fail(
                "enrollment",
                if enrolled.is_empty() {
                    "none"
                } else {
                    "ambiguous"
                },
            ),
            None,
        );
    };
    report.note("enrollment", "observed");

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
        Err(error) => return (report.fail("policy", format!("{error:?}")), None),
    };
    let candidates: Vec<_> = inventory
        .iter()
        .zip(&digests)
        .map(|(observed, digest)| observed.candidate(digest))
        .collect();
    let targets = match policy.targets(&env, &candidates) {
        Ok(targets) => targets,
        Err(error) => return (report.fail("targets", format!("{error:?}")), None),
    };
    let [target] = targets.as_slice() else {
        return (report.fail("targets", targets.len()), None);
    };
    if target.instance() != observed.instance {
        return (report.fail("targets", "other_instance"), None);
    }
    report.note("targets", 1);

    let opened = match Opened::open(&dev, observed) {
        Ok(opened) => opened,
        Err(error) => {
            return (
                report.fail("open", format!("error:{:?}", error.kind())),
                None,
            );
        }
    };
    report.note("open", "ok");

    let fresh = match inventory::scan(&sys, &dev) {
        Ok(fresh) => fresh,
        Err(error) => {
            return (
                report.fail("rescan", format!("error:{:?}", error.kind())),
                None,
            );
        }
    };
    let fresh_digests = node_digests(root, &fresh, layout_sha256);
    let fresh: Vec<_> = fresh
        .iter()
        .zip(&fresh_digests)
        .map(|(observed, digest)| observed.candidate(digest))
        .collect();
    match policy.revalidate(
        target,
        &opened.candidate(observed, layout_sha256),
        &env,
        &fresh,
    ) {
        Ok(true) => report.note("revalidate", "ok"),
        Ok(false) => return (report.fail("revalidate", "mismatch"), None),
        Err(error) => return (report.fail("revalidate", format!("{error:?}")), None),
    }

    match opened.pressed_keys() {
        Ok(keys) if keys.is_empty() => report.note("neutral", "yes"),
        Ok(keys) => return (report.fail("neutral", format!("no:{}", keys.len())), None),
        Err(error) => {
            return (
                report.fail("neutral", format!("error:{:?}", error.kind())),
                None,
            );
        }
    }
    report.note("check", "pass");
    report.passed = true;
    report.checked = Some(Checked {
        evidence,
        input_id: opened.input_id,
    });
    (report, Some(opened))
}

/// The `Generic.kl` digest for each node, or a non-matching marker where a device-specific layout applies.
fn node_digests<'a>(root: &Path, nodes: &[Observation], generic: &'a str) -> Vec<&'a str> {
    nodes
        .iter()
        .map(|node| {
            if layout::has_override(root, node) {
                OVERRIDDEN
            } else {
                generic
            }
        })
        .collect()
}
