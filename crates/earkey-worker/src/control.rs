//! Root command operations over the private store.
//!
//! Enrollment comes only from a unique live observation that matches reviewed
//! evidence. Policy writes keep the saved generation and recovery state, and
//! report only the saved generation; the worker acknowledges the active one.
//! Status never includes serials. It lists each enrolled accessory's measured
//! controls by selection name with their disclosed effects.

use crate::environment::{Observed, evidence_for};
use crate::inventory;
use crate::layout;
use crate::selection::Selection;
use earkey_policy::identity::{Evidence, VerifiedEnrollment};
use earkey_policy::policy::{Policy, RecoveryLatch};
use earkey_policy::store::{EnrollmentRecord, RecoveryOperation, Runtime, Store, StoreError};
use std::io;
use std::path::Path;

#[derive(Debug)]
pub enum ControlError {
    Store(StoreError),
    Layout(io::ErrorKind),
    Inventory(io::ErrorKind),
    UnknownEnvironment,
    /// Each scanned node with the first check it failed.
    NoCandidate(String),
    AmbiguousCandidates,
    NotEnrolled,
    Json,
}

impl From<StoreError> for ControlError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

/// Enrolls the single live candidate matching the environment's evidence.
pub fn enroll(
    store: &Store,
    root: &Path,
    environment: &Observed,
) -> Result<Evidence, ControlError> {
    let evidence = evidence_for(environment).ok_or(ControlError::UnknownEnvironment)?;
    let generic =
        layout::generic_digest(root).map_err(|error| ControlError::Layout(error.kind()))?;
    let nodes = inventory::scan(&root.join("sys"), &root.join("dev"))
        .map_err(|error| ControlError::Inventory(error.kind()))?;
    let env = environment.as_environment();
    let mut enrolled = nodes
        .iter()
        .filter(|node| !layout::has_override(root, node))
        .filter_map(|node| {
            VerifiedEnrollment::from_observation(evidence, &env, &node.candidate(&generic))
        });
    let enrollment = enrolled.next().ok_or_else(|| {
        let reasons: Vec<String> = nodes
            .iter()
            .map(|node| {
                let reason = if layout::has_override(root, node) {
                    "override"
                } else {
                    evidence
                        .mismatch(&env, &node.candidate(&generic))
                        .unwrap_or("none")
                };
                format!("{}={reason}", node.event)
            })
            .collect();
        ControlError::NoCandidate(reasons.join(","))
    })?;
    if enrolled.next().is_some() {
        return Err(ControlError::AmbiguousCandidates);
    }
    store.enroll(&enrollment)?;
    Ok(evidence)
}

fn verified(records: &[EnrollmentRecord]) -> Vec<VerifiedEnrollment<'_>> {
    records
        .iter()
        .filter_map(EnrollmentRecord::verified)
        .collect()
}

/// Saves a policy blocking `selection` for every enrolled accessory, and returns the saved generation.
/// An accessory re-enrolled under newer evidence uses its latest enrollment.
pub fn apply(store: &Store, selection: &Selection, enabled: bool) -> Result<u64, ControlError> {
    let records = store.enrollments()?;
    if records.is_empty() {
        return Err(ControlError::NotEnrolled);
    }
    let verified = verified(&records);
    let current = store.load(&verified)?;
    let mut latest: Vec<&EnrollmentRecord> = Vec::new();
    for record in records.iter().rev() {
        if !latest
            .iter()
            .any(|seen| seen.usb_serial == record.usb_serial)
        {
            latest.insert(0, record);
        }
    }
    let profiles: Vec<serde_json::Value> = latest
        .into_iter()
        .map(|record| {
            let groups = match selection {
                Selection::BlockAll => vec![],
                _ => selection.groups(record.evidence),
            };
            serde_json::json!({
                "evidence": record.evidence,
                "usb_serial": record.usb_serial,
                "enabled": true,
                "blocked_groups": groups,
            })
        })
        .collect();
    let request = serde_json::json!({
        "schema_version": 1,
        "generation": current.generation(),
        "enabled": enabled,
        "preset": if *selection == Selection::BlockAll { "block_all" } else { "custom" },
        "profiles": profiles,
        "requested_backend": "evdev_uinput",
        "unknown_accessory": "ignore",
        "diagnostics": {"enabled": false, "seconds": 0, "max_bytes": 0},
        "recovery": current.recovery(),
    });
    let bytes = serde_json::to_vec(&request).map_err(|_| ControlError::Json)?;
    Ok(store.save(&bytes, &verified)?.generation())
}

/// Changes only the master enabled state, and returns the saved generation.
pub fn set_enabled(store: &Store, enabled: bool) -> Result<u64, ControlError> {
    let records = store.enrollments()?;
    let verified = verified(&records);
    let current = store.load(&verified)?;
    let mut value: serde_json::Value =
        serde_json::from_slice(&current.to_json()).map_err(|_| ControlError::Json)?;
    value["enabled"] = enabled.into();
    let bytes = serde_json::to_vec(&value).map_err(|_| ControlError::Json)?;
    Ok(store.save(&bytes, &verified)?.generation())
}

/// Applies a dedicated recovery operation at the current generation.
pub fn recovery(store: &Store, operation: RecoveryOperation) -> Result<u64, ControlError> {
    let records = store.enrollments()?;
    let verified = verified(&records);
    let generation = store.load(&verified)?.generation();
    Ok(store
        .apply_recovery(generation, operation, &verified)?
        .generation())
}

/// Saved state, then runtime reports, as `key=value` pairs, without serials.
pub fn status(store: &Store) -> Vec<(&'static str, String)> {
    let mut lines = saved(store);
    for (key, file) in [
        ("worker", Runtime::Worker),
        ("supervisor", Runtime::Supervisor),
    ] {
        match store.read_runtime(file) {
            Ok(Some(text)) => lines.extend(text.lines().map(|line| (key, line.to_owned()))),
            Ok(None) => lines.push((key, "none".into())),
            Err(error) => lines.push((key, format!("error:{error:?}"))),
        }
    }
    lines
}

fn saved(store: &Store) -> Vec<(&'static str, String)> {
    let mut lines = Vec::new();
    let records = match store.enrollments() {
        Ok(records) => records,
        Err(error) => {
            lines.push(("enrollments", format!("error:{error:?}")));
            return lines;
        }
    };
    let evidence: Vec<String> = records
        .iter()
        .map(|record| {
            serde_json::to_value(record.evidence)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        })
        .collect();
    lines.push(("enrollments", records.len().to_string()));
    lines.push((
        "enrolled_evidence",
        if evidence.is_empty() {
            "none".into()
        } else {
            evidence.join(",")
        },
    ));
    let mut listed = Vec::new();
    for control in records.iter().flat_map(|record| record.evidence.controls()) {
        if listed.contains(&control.group) {
            continue;
        }
        listed.push(control.group);
        let name = serde_json::to_value(control.group)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.replace('_', "-")))
            .unwrap_or_default();
        let effects: Vec<&str> = control
            .observed_effects
            .iter()
            .chain(control.reported_effects)
            .copied()
            .collect();
        lines.push(("control", format!("{name}:{}", effects.join("|"))));
    }
    lines.push((
        "stay_inactive",
        store.has_inactive_marker().map_or_else(
            |error| format!("error:{error:?}"),
            |marker| marker.to_string(),
        ),
    ));
    let verified = verified(&records);
    let policy: Policy = match store.load(&verified) {
        Ok(policy) => policy,
        Err(error) => {
            lines.push(("policy", format!("error:{error:?}")));
            return lines;
        }
    };
    let recovery = policy.recovery();
    lines.push(("saved_generation", policy.generation().to_string()));
    lines.push(("enabled", policy.enabled().to_string()));
    lines.push(("paused", recovery.paused.to_string()));
    lines.push(("boot_disabled", recovery.boot_disabled.to_string()));
    lines.push((
        "restart_latch",
        (recovery.latch == RecoveryLatch::RestartLimit).to_string(),
    ));
    lines.push(("profiles", policy.profile_count().to_string()));
    for index in 0..policy.profile_count() {
        let groups: Vec<String> = policy
            .blocked_controls(index)
            .map(|control| format!("{:?}", control.group))
            .collect();
        lines.push((
            "profile_blocked_groups",
            if groups.is_empty() {
                "none".into()
            } else {
                groups.join(",")
            },
        ));
    }
    lines
}
