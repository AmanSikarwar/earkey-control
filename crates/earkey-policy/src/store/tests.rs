use super::*;
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
