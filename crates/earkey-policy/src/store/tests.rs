use super::*;
use crate::identity::Evidence;
use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "earkey-policy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }

    fn store(&self) -> Store {
        let dir = File::open(&self.0).unwrap();
        let owner = dir.metadata().unwrap().uid();
        Store::from_directory(dir, owner).unwrap()
    }

    fn write_policy(&self, bytes: &[u8]) {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.0.join("policy.json"))
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const SERIAL: &str = "SYNTHETIC-ENROLLMENT";
const REQUEST: &str = include_str!("../../../../tests/fixtures/identity/reference-policy.json");

fn enrollment() -> VerifiedEnrollment<'static> {
    VerifiedEnrollment::from_reviewed_measurement(Evidence::XiaomiPixelos20261107, SERIAL).unwrap()
}

#[cfg(feature = "reforge-passthrough")]
#[test]
fn diagnostic_evidence_cannot_be_loaded_or_enrolled() {
    let scratch = Scratch::new();
    let evidence = Evidence::XiaomiPixelosReforge20261111;
    let verified = VerifiedEnrollment::from_reviewed_measurement(evidence, SERIAL).unwrap();
    let bytes = Policy::for_acquisition(&verified).to_json();
    assert!(matches!(
        Policy::parse(&bytes, std::slice::from_ref(&verified)),
        Err(PolicyError::InvalidJson)
    ));
    assert!(matches!(
        scratch.store().enroll(&verified),
        Err(StoreError::InvalidEnrollments)
    ));
    assert!(scratch.store().enrollments().unwrap().is_empty());
}

#[test]
fn missing_is_inactive_and_valid_save_assigns_next_generation() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let initial = store.load(&[]).unwrap();
    assert_eq!(initial.generation(), 0);
    assert_eq!(initial.to_json(), Policy::inactive().to_json());

    let saved = store.save(&initial.to_json(), &[]).unwrap();
    assert_eq!(saved.generation(), 1);
    assert_eq!(store.load(&[]).unwrap().to_json(), saved.to_json());
    assert_eq!(
        fs::metadata(scratch.0.join("policy.json")).unwrap().mode() & 0o7777,
        0o600
    );
    assert!(!scratch.0.join("policy.tmp").exists());
    assert!(matches!(
        store.save(&initial.to_json(), &[]),
        Err(StoreError::StaleGeneration {
            requested: 0,
            saved: 1
        })
    ));
    assert!(matches!(
        store.save(b"{}", &[]),
        Err(StoreError::Policy(PolicyError::InvalidJson))
    ));
    assert_eq!(store.load(&[]).unwrap().to_json(), saved.to_json());
}

#[test]
fn concurrent_writers_cannot_both_commit_same_base() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let request = Policy::inactive().to_json();
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| store.save(&request, &[]));
        let second = scope.spawn(|| store.save(&request, &[]));
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(StoreError::StaleGeneration { .. })))
            .count(),
        1
    );
    assert_eq!(store.load(&[]).unwrap().generation(), 1);
}

#[test]
fn policy_edits_preserve_existing_recovery_state() {
    let scratch = Scratch::new();
    let mut state: serde_json::Value =
        serde_json::from_slice(&Policy::inactive().to_json()).unwrap();
    state["generation"] = 7.into();
    state["recovery"]["paused"] = true.into();
    state["recovery"]["boot_disabled"] = true.into();
    state["recovery"]["latch"] = "restart_limit".into();
    scratch.write_policy(&serde_json::to_vec(&state).unwrap());
    let store = scratch.store();
    let saved = store.load(&[]).unwrap();
    state["recovery"]["paused"] = false.into();
    assert!(matches!(
        store.save(&serde_json::to_vec(&state).unwrap(), &[]),
        Err(StoreError::RecoveryChange)
    ));

    state["recovery"]["paused"] = true.into();
    state["preset"] = "block_all".into();
    let next = store
        .save(&serde_json::to_vec(&state).unwrap(), &[])
        .unwrap();
    assert_eq!(next.generation(), 8);
    assert_eq!(next.recovery(), saved.recovery());
}

#[test]
fn corrupt_or_future_policy_cannot_be_overwritten_by_default() {
    for (bytes, expected) in [
        (
            br#"{ "schema_version": 42 }"#.as_slice(),
            PolicyError::UnsupportedSchema(42),
        ),
        (
            br#"{ "schema_version": 1 "#.as_slice(),
            PolicyError::InvalidJson,
        ),
    ] {
        let scratch = Scratch::new();
        scratch.write_policy(bytes);
        let store = scratch.store();
        assert!(
            matches!(store.load(&[]), Err(StoreError::Policy(ref error)) if error == &expected)
        );
        assert!(
            matches!(store.save(&Policy::inactive().to_json(), &[]), Err(StoreError::Policy(ref error)) if error == &expected)
        );
        assert_eq!(fs::read(scratch.0.join("policy.json")).unwrap(), bytes);
    }
}

#[test]
fn unsafe_directory_and_file_entries_are_rejected() {
    let scratch = Scratch::new();
    let store = scratch.store();
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(store.load(&[]), Err(StoreError::UnsafeStorage)));
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o700)).unwrap();

    let policy = scratch.0.join("policy.json");
    let sentinel = scratch.0.join("sentinel");
    fs::write(&sentinel, b"sentinel").unwrap();
    fs::set_permissions(&sentinel, fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&sentinel, &policy).unwrap();
    assert!(store.load(&[]).is_err());
    assert!(store.save(&Policy::inactive().to_json(), &[]).is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"sentinel");
    fs::remove_file(&policy).unwrap();

    fs::hard_link(&sentinel, &policy).unwrap();
    assert!(matches!(store.load(&[]), Err(StoreError::UnsafeStorage)));
    fs::remove_file(&policy).unwrap();
    fs::create_dir(&policy).unwrap();
    assert!(matches!(store.load(&[]), Err(StoreError::UnsafeStorage)));
    fs::remove_dir(&policy).unwrap();
    fs::write(&policy, b"{}").unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(store.load(&[]), Err(StoreError::UnsafeStorage)));
}

#[test]
fn unsafe_temp_does_not_replace_last_good_file() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let saved = store.save(&Policy::inactive().to_json(), &[]).unwrap();
    let temp = scratch.0.join("policy.tmp");
    symlink("policy.json", &temp).unwrap();
    assert!(store.save(&saved.to_json(), &[]).is_err());
    assert_eq!(store.load(&[]).unwrap().to_json(), saved.to_json());
    fs::remove_file(&temp).unwrap();

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)
        .unwrap()
        .write_all(b"interrupted write")
        .unwrap();
    assert_eq!(store.save(&saved.to_json(), &[]).unwrap().generation(), 2);
    assert!(!temp.exists());
}

#[test]
fn recovery_operations_are_generation_checked_and_preserve_policy() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let enrollments = [enrollment()];
    let saved = store.save(REQUEST.as_bytes(), &enrollments).unwrap();
    assert_eq!(saved.generation(), 1);
    assert!(!saved.recovery().paused);
    assert_eq!(saved.blocked_controls(0).count(), 1);

    assert!(matches!(
        store.apply_recovery(0, RecoveryOperation::Pause, &enrollments),
        Err(StoreError::StaleGeneration {
            requested: 0,
            saved: 1
        })
    ));
    assert!(!store.load(&enrollments).unwrap().recovery().paused);

    let paused = store
        .apply_recovery(1, RecoveryOperation::Pause, &enrollments)
        .unwrap();
    assert_eq!(paused.generation(), 2);
    assert!(paused.recovery().paused);
    assert_eq!(paused.blocked_controls(0).count(), 1);

    let reloaded = store.load(&enrollments).unwrap();
    assert_eq!(reloaded.recovery(), paused.recovery());
    assert_eq!(reloaded.blocked_controls(0).count(), 1);

    let mut ordinary = serde_json::from_slice::<serde_json::Value>(&paused.to_json()).unwrap();
    ordinary["recovery"]["paused"] = false.into();
    assert!(matches!(
        store.save(&serde_json::to_vec(&ordinary).unwrap(), &enrollments),
        Err(StoreError::RecoveryChange)
    ));
    assert!(store.load(&enrollments).unwrap().recovery().paused);

    let resumed = store
        .apply_recovery(2, RecoveryOperation::Resume, &enrollments)
        .unwrap();
    assert_eq!(resumed.generation(), 3);
    assert!(!resumed.recovery().paused);
    assert_eq!(resumed.blocked_controls(0).count(), 1);
}

#[test]
fn boot_disable_and_enable_manage_the_preopen_marker() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let marker = scratch.0.join("stay-inactive");
    assert!(!store.has_inactive_marker().unwrap());

    let disabled = store
        .apply_recovery(0, RecoveryOperation::DisableNextBoot, &[])
        .unwrap();
    assert_eq!(disabled.generation(), 1);
    assert!(disabled.recovery().boot_disabled);
    assert!(store.has_inactive_marker().unwrap());
    let metadata = fs::metadata(&marker).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.nlink(), 1);

    let mut ordinary = serde_json::from_slice::<serde_json::Value>(&disabled.to_json()).unwrap();
    ordinary["recovery"]["boot_disabled"] = false.into();
    assert!(matches!(
        store.save(&serde_json::to_vec(&ordinary).unwrap(), &[]),
        Err(StoreError::RecoveryChange)
    ));
    assert!(store.has_inactive_marker().unwrap());

    let enabled = store
        .apply_recovery(1, RecoveryOperation::EnableNextBoot, &[])
        .unwrap();
    assert_eq!(enabled.generation(), 2);
    assert!(!enabled.recovery().boot_disabled);
    assert!(!store.has_inactive_marker().unwrap());
    assert!(!marker.exists());
}

#[test]
fn restart_latch_requires_an_explicit_recovery_operation() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let saved = store.save(&Policy::inactive().to_json(), &[]).unwrap();

    let mut ordinary = serde_json::from_slice::<serde_json::Value>(&saved.to_json()).unwrap();
    ordinary["recovery"]["latch"] = "restart_limit".into();
    assert!(matches!(
        store.save(&serde_json::to_vec(&ordinary).unwrap(), &[]),
        Err(StoreError::RecoveryChange)
    ));

    let latched = store
        .apply_recovery(1, RecoveryOperation::LatchRestartLimit, &[])
        .unwrap();
    assert_eq!(latched.generation(), 2);
    assert_eq!(latched.recovery().latch, RecoveryLatch::RestartLimit);

    let cleared = store
        .apply_recovery(2, RecoveryOperation::ClearRestartLimit, &[])
        .unwrap();
    assert_eq!(cleared.generation(), 3);
    assert_eq!(cleared.recovery().latch, RecoveryLatch::Clear);
}

#[test]
fn tampered_marker_blocks_boot_recovery_without_changing_policy() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let saved = store.save(&Policy::inactive().to_json(), &[]).unwrap();
    let marker = scratch.0.join("stay-inactive");

    symlink("missing", &marker).unwrap();
    for operation in [
        RecoveryOperation::DisableNextBoot,
        RecoveryOperation::EnableNextBoot,
    ] {
        assert!(matches!(
            store.apply_recovery(saved.generation(), operation, &[]),
            Err(StoreError::UnsafeStorage)
        ));
    }
    assert_eq!(store.load(&[]).unwrap().to_json(), saved.to_json());
    assert!(
        fs::symlink_metadata(&marker)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_file(&marker).unwrap();

    fs::write(&marker, b"").unwrap();
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o644)).unwrap();
    for operation in [
        RecoveryOperation::DisableNextBoot,
        RecoveryOperation::EnableNextBoot,
    ] {
        assert!(matches!(
            store.apply_recovery(saved.generation(), operation, &[]),
            Err(StoreError::UnsafeStorage)
        ));
    }
    assert_eq!(store.load(&[]).unwrap().to_json(), saved.to_json());
}

#[test]
fn failed_boot_disable_keeps_the_marker_before_policy_retry() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let saved = store.save(&Policy::inactive().to_json(), &[]).unwrap();
    let marker = scratch.0.join("stay-inactive");
    let temp = scratch.0.join("policy.tmp");
    symlink("policy.json", &temp).unwrap();

    assert!(matches!(
        store.apply_recovery(1, RecoveryOperation::DisableNextBoot, &[]),
        Err(StoreError::UnsafeStorage)
    ));
    assert!(store.has_inactive_marker().unwrap());
    let unchanged = store.load(&[]).unwrap();
    assert_eq!(unchanged.generation(), saved.generation());
    assert!(!unchanged.recovery().boot_disabled);
    fs::remove_file(&temp).unwrap();

    let disabled = store
        .apply_recovery(1, RecoveryOperation::DisableNextBoot, &[])
        .unwrap();
    assert_eq!(disabled.generation(), 2);
    assert!(disabled.recovery().boot_disabled);
    assert!(marker.exists());
}

#[test]
fn recovery_operations_still_require_verified_enrollments() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let enrollments = [enrollment()];
    let saved = store.save(REQUEST.as_bytes(), &enrollments).unwrap();

    assert!(matches!(
        store.apply_recovery(saved.generation(), RecoveryOperation::Pause, &[]),
        Err(StoreError::Policy(PolicyError::UnverifiedProfile))
    ));
    assert_eq!(store.load(&enrollments).unwrap().to_json(), saved.to_json());
}

#[test]
fn concurrent_recovery_operations_cannot_both_commit_same_base() {
    let scratch = Scratch::new();
    let store = scratch.store();
    store.save(&Policy::inactive().to_json(), &[]).unwrap();
    let results = std::thread::scope(|scope| {
        let pause = scope.spawn(|| store.apply_recovery(1, RecoveryOperation::Pause, &[]));
        let latch =
            scope.spawn(|| store.apply_recovery(1, RecoveryOperation::LatchRestartLimit, &[]));
        [pause.join().unwrap(), latch.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(StoreError::StaleGeneration { .. })))
            .count(),
        1
    );
    assert_eq!(store.load(&[]).unwrap().generation(), 2);
}

#[test]
fn exhausted_generation_does_not_create_a_boot_marker() {
    let scratch = Scratch::new();
    let mut state: serde_json::Value =
        serde_json::from_slice(&Policy::inactive().to_json()).unwrap();
    state["generation"] = u64::MAX.into();
    scratch.write_policy(&serde_json::to_vec(&state).unwrap());
    let store = scratch.store();

    assert!(matches!(
        store.apply_recovery(u64::MAX, RecoveryOperation::DisableNextBoot, &[]),
        Err(StoreError::GenerationExhausted)
    ));
    assert!(!store.has_inactive_marker().unwrap());
}

#[test]
fn inactive_marker_must_be_absent_or_a_private_regular_file() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let marker = scratch.0.join("stay-inactive");
    assert!(!store.has_inactive_marker().unwrap());

    symlink("missing", &marker).unwrap();
    assert!(matches!(
        store.has_inactive_marker(),
        Err(StoreError::UnsafeStorage)
    ));
    fs::remove_file(&marker).unwrap();

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&marker)
        .unwrap();
    assert!(store.has_inactive_marker().unwrap());
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        store.has_inactive_marker(),
        Err(StoreError::UnsafeStorage)
    ));
}

#[test]
fn recovery_read_skips_only_enrollment_matching() {
    let scratch = Scratch::new();
    let store = scratch.store();
    assert_eq!(store.recovery().unwrap(), Policy::inactive().recovery());

    let enrollments = [enrollment()];
    store.save(REQUEST.as_bytes(), &enrollments).unwrap();
    store
        .apply_recovery(1, RecoveryOperation::Pause, &enrollments)
        .unwrap();
    assert!(matches!(
        store.load(&[]),
        Err(StoreError::Policy(PolicyError::UnverifiedProfile))
    ));
    let recovery = store.recovery().unwrap();
    assert!(recovery.paused);
    assert!(!recovery.boot_disabled);
    assert_eq!(recovery.latch, RecoveryLatch::Clear);
}

#[test]
fn recovery_read_rejects_corrupt_unsafe_and_unsaved_policies() {
    for (bytes, expected) in [
        (
            br#"{ "schema_version": 42 }"#.as_slice(),
            PolicyError::UnsupportedSchema(42),
        ),
        (
            br#"{ "schema_version": 1 "#.as_slice(),
            PolicyError::InvalidJson,
        ),
    ] {
        let scratch = Scratch::new();
        scratch.write_policy(bytes);
        assert!(
            matches!(scratch.store().recovery(), Err(StoreError::Policy(ref error)) if error == &expected)
        );
    }

    let scratch = Scratch::new();
    scratch.write_policy(&Policy::inactive().to_json());
    assert!(matches!(
        scratch.store().recovery(),
        Err(StoreError::UnsafeStorage)
    ));
    let policy = scratch.0.join("policy.json");
    fs::remove_file(&policy).unwrap();
    symlink("missing", &policy).unwrap();
    assert!(scratch.store().recovery().is_err());
    fs::remove_file(&policy).unwrap();

    let store = scratch.store();
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(store.recovery(), Err(StoreError::UnsafeStorage)));
    assert!(matches!(
        store.latch_restart_limit(),
        Err(StoreError::UnsafeStorage)
    ));
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn restart_latch_write_preserves_profiles_without_enrollments() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let enrollments = [enrollment()];
    let saved = store.save(REQUEST.as_bytes(), &enrollments).unwrap();

    store.latch_restart_limit().unwrap();
    let latched = store.load(&enrollments).unwrap();
    assert_eq!(latched.generation(), 2);
    assert_eq!(latched.recovery().latch, RecoveryLatch::RestartLimit);
    assert_eq!(
        latched.blocked_controls(0).count(),
        saved.blocked_controls(0).count()
    );

    store.latch_restart_limit().unwrap();
    assert_eq!(store.load(&enrollments).unwrap().generation(), 2);
    assert!(matches!(
        store.save(&latched.to_json(), &enrollments),
        Ok(ref policy) if policy.recovery().latch == RecoveryLatch::RestartLimit
    ));
}

fn write_enrollments(scratch: &Scratch, bytes: &[u8]) {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(scratch.0.join("enrollments.json"))
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

#[test]
fn enrollments_round_trip_and_verify_saved_policies() {
    let scratch = Scratch::new();
    let store = scratch.store();
    assert!(store.enrollments().unwrap().is_empty());

    store.enroll(&enrollment()).unwrap();
    store.enroll(&enrollment()).unwrap();
    let records = store.enrollments().unwrap();
    assert_eq!(
        records,
        [EnrollmentRecord {
            evidence: Evidence::XiaomiPixelos20261107,
            usb_serial: SERIAL.into(),
        }]
    );
    let metadata = fs::metadata(scratch.0.join("enrollments.json")).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert!(!scratch.0.join("enrollments.tmp").exists());
    assert_eq!(
        fs::read(scratch.0.join("enrollments.json")).unwrap(),
        format!(
            r#"{{"schema_version":1,"enrollments":[{{"evidence":"xiaomi_pixelos20261107","usb_serial":"{SERIAL}"}}]}}"#
        )
        .as_bytes()
    );

    let verified: Vec<_> = records
        .iter()
        .filter_map(EnrollmentRecord::verified)
        .collect();
    store.save(REQUEST.as_bytes(), &verified).unwrap();
    assert_eq!(store.load(&verified).unwrap().generation(), 1);
    assert!(store.save(REQUEST.as_bytes(), &[]).is_err());
}

#[test]
fn malformed_enrollment_records_are_rejected() {
    let record = |serial: &str| {
        format!(r#"{{"evidence":"xiaomi_pixelos20261107","usb_serial":"{serial}"}}"#)
    };
    let file = |schema: u32, records: &[String]| {
        format!(
            r#"{{"schema_version":{schema},"enrollments":[{}]}}"#,
            records.join(",")
        )
    };
    let nine: Vec<_> = (0..9).map(|n| record(&format!("S{n}"))).collect();
    for (bytes, expected) in [
        (file(2, &[record("A")]), "InvalidEnrollments"),
        (file(1, &[record("A"), record("A")]), "InvalidEnrollments"),
        (file(1, &[record(" ")]), "InvalidEnrollments"),
        (file(1, &[record("A\\u0007")]), "InvalidEnrollments"),
        (
            file(1, &[record("A")]).replace("}]", r#","extra":1}]"#),
            "InvalidEnrollments",
        ),
        (
            r#"{"schema_version":1,"schema_version":1,"enrollments":[]}"#.into(),
            "InvalidEnrollments",
        ),
        (file(1, &nine), "TooManyEnrollments"),
        ("{".into(), "InvalidEnrollments"),
    ] {
        let scratch = Scratch::new();
        write_enrollments(&scratch, bytes.as_bytes());
        let error = scratch.store().enrollments().unwrap_err();
        assert_eq!(format!("{error:?}"), expected, "{bytes}");
        assert!(scratch.store().enroll(&enrollment()).is_err());
        assert_eq!(
            fs::read(scratch.0.join("enrollments.json")).unwrap(),
            bytes.as_bytes()
        );
    }
}

#[test]
fn enrollment_capacity_and_unsafe_files_are_enforced() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let serials: Vec<String> = (0..MAX_ENROLLMENTS).map(|n| format!("S{n}")).collect();
    for serial in &serials {
        let entry =
            VerifiedEnrollment::from_reviewed_measurement(Evidence::XiaomiPixelos20261107, serial)
                .unwrap();
        store.enroll(&entry).unwrap();
    }
    assert!(matches!(
        store.enroll(&enrollment()),
        Err(StoreError::TooManyEnrollments)
    ));
    assert_eq!(store.enrollments().unwrap().len(), MAX_ENROLLMENTS);

    let scratch = Scratch::new();
    let path = scratch.0.join("enrollments.json");
    symlink("missing", &path).unwrap();
    assert!(scratch.store().enrollments().is_err());
    assert!(scratch.store().enroll(&enrollment()).is_err());
    fs::remove_file(&path).unwrap();
    fs::write(&path, br#"{"schema_version":1,"enrollments":[]}"#).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        scratch.store().enrollments(),
        Err(StoreError::UnsafeStorage)
    ));
}

#[test]
fn runtime_status_files_are_bounded_private_and_separate() {
    let scratch = Scratch::new();
    let store = scratch.store();
    assert_eq!(store.read_runtime(Runtime::Worker).unwrap(), None);
    store
        .write_runtime(Runtime::Worker, "state=waiting_accessory\n")
        .unwrap();
    store
        .write_runtime(Runtime::Supervisor, "phase=Running\n")
        .unwrap();
    store
        .write_runtime(Runtime::Worker, "state=active\n")
        .unwrap();
    assert_eq!(
        store.read_runtime(Runtime::Worker).unwrap().as_deref(),
        Some("state=active\n")
    );
    assert_eq!(
        store.read_runtime(Runtime::Supervisor).unwrap().as_deref(),
        Some("phase=Running\n")
    );
    let metadata = fs::metadata(scratch.0.join("worker.status")).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert!(!scratch.0.join("worker.status.tmp").exists());
    assert_eq!(store.load(&[]).unwrap().generation(), 0);

    assert!(matches!(
        store.write_runtime(Runtime::Worker, &"x".repeat(MAX_RUNTIME_BYTES + 1)),
        Err(StoreError::InvalidRuntime)
    ));
    fs::set_permissions(
        scratch.0.join("supervisor.status"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(matches!(
        store.read_runtime(Runtime::Supervisor),
        Err(StoreError::UnsafeStorage)
    ));
    fs::remove_file(scratch.0.join("supervisor.status")).unwrap();
    symlink("worker.status", scratch.0.join("supervisor.status")).unwrap();
    assert!(store.read_runtime(Runtime::Supervisor).is_err());
}
