use crate::identity::{Evidence, VerifiedEnrollment};
use crate::policy::{MAX_POLICY_BYTES, Policy, PolicyError, Recovery, RecoveryLatch};
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

const POLICY: &CStr = c"policy.json";
const TEMP: &CStr = c"policy.tmp";
const STAY_INACTIVE: &CStr = c"stay-inactive";
const ENROLLMENTS: &CStr = c"enrollments.json";
const ENROLLMENTS_TEMP: &CStr = c"enrollments.tmp";
/// Maximum size of a generated runtime status file.
pub const MAX_RUNTIME_BYTES: usize = 4096;

/// Generated runtime status, kept apart from durable policy and enrollments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Runtime {
    Worker,
    Supervisor,
}

impl Runtime {
    fn names(self) -> (&'static CStr, &'static CStr) {
        match self {
            Self::Worker => (c"worker.status", c"worker.status.tmp"),
            Self::Supervisor => (c"supervisor.status", c"supervisor.status.tmp"),
        }
    }
}
const ENROLLMENT_SCHEMA: u32 = 1;
/// Maximum durable enrollments; equal to the policy profile limit.
pub const MAX_ENROLLMENTS: usize = 8;
const DIRECTORY: libc::c_int = libc::O_RDONLY | libc::O_DIRECTORY;

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    UnsafeStorage,
    Policy(PolicyError),
    StaleGeneration {
        requested: u64,
        saved: u64,
    },
    GenerationExhausted,
    RecoveryChange,
    DurabilityUncertain {
        generation: u64,
        source: io::Error,
    },
    RecoveryIncomplete {
        generation: u64,
        source: Box<StoreError>,
    },
    InvalidEnrollments,
    TooManyEnrollments,
    InvalidRuntime,
}

/// A durable enrollment written only by [`Store::enroll`] from a live observation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRecord {
    pub evidence: Evidence,
    pub usb_serial: String,
}

impl EnrollmentRecord {
    /// Records read from the store were validated when parsed.
    pub fn verified(&self) -> Option<VerifiedEnrollment<'_>> {
        VerifiedEnrollment::from_reviewed_measurement(self.evidence, &self.usb_serial)
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Enrollments {
    schema_version: u32,
    enrollments: Vec<EnrollmentRecord>,
}

fn parse_enrollments(bytes: &[u8]) -> Result<Vec<EnrollmentRecord>, StoreError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| StoreError::InvalidEnrollments)?;
    let parsed: Enrollments =
        serde_json::from_slice(bytes).map_err(|_| StoreError::InvalidEnrollments)?;
    if parsed.schema_version != ENROLLMENT_SCHEMA
        || serde_json::to_value(&parsed).map_err(|_| StoreError::InvalidEnrollments)? != value
    {
        return Err(StoreError::InvalidEnrollments);
    }
    let records = parsed.enrollments;
    if records.len() > MAX_ENROLLMENTS {
        return Err(StoreError::TooManyEnrollments);
    }
    for (index, record) in records.iter().enumerate() {
        if record.verified().is_none() || records[..index].contains(record) {
            return Err(StoreError::InvalidEnrollments);
        }
    }
    Ok(records)
}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<PolicyError> for StoreError {
    fn from(error: PolicyError) -> Self {
        Self::Policy(error)
    }
}

/// A dedicated recovery mutation. Ordinary policy saves cannot change recovery state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryOperation {
    Pause,
    Resume,
    DisableNextBoot,
    EnableNextBoot,
    LatchRestartLimit,
    ClearRestartLimit,
}

impl RecoveryOperation {
    fn apply(self, mut recovery: Recovery) -> Recovery {
        match self {
            Self::Pause => recovery.paused = true,
            Self::Resume => recovery.paused = false,
            Self::DisableNextBoot => recovery.boot_disabled = true,
            Self::EnableNextBoot => recovery.boot_disabled = false,
            Self::LatchRestartLimit => recovery.latch = RecoveryLatch::RestartLimit,
            Self::ClearRestartLimit => recovery.latch = RecoveryLatch::Clear,
        }
        recovery
    }
}

pub struct Store {
    directory: File,
    owner: u32,
}

impl Store {
    /// Opens manager-independent storage provisioned by the installer.
    pub fn open() -> Result<Self, StoreError> {
        // SAFETY: geteuid has no arguments or memory preconditions.
        if unsafe { libc::geteuid() } != 0 {
            return Err(StoreError::UnsafeStorage);
        }
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open("/")?;
        let data = open_at(&root, c"data", DIRECTORY)?;
        let adb = open_at(&data, c"adb", DIRECTORY)?;
        let metadata = adb.metadata()?;
        if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err(StoreError::UnsafeStorage);
        }
        Self::from_directory(open_at(&adb, c"earkey_control", DIRECTORY)?, 0)
    }

    /// Opens a private directory owned by the effective user, for tests and alternate roots.
    /// It applies the same `0700` and private-file checks; production uses [`Store::open`].
    pub fn open_dir(path: &std::path::Path) -> Result<Self, StoreError> {
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)?;
        // SAFETY: geteuid has no arguments or memory preconditions.
        Self::from_directory(directory, unsafe { libc::geteuid() })
    }

    fn from_directory(directory: File, owner: u32) -> Result<Self, StoreError> {
        let store = Self { directory, owner };
        store.check_directory()?;
        Ok(store)
    }

    fn check_directory(&self) -> Result<(), StoreError> {
        let metadata = self.directory.metadata()?;
        if !metadata.is_dir() || metadata.uid() != self.owner || metadata.mode() & 0o7777 != 0o700 {
            return Err(StoreError::UnsafeStorage);
        }
        Ok(())
    }

    /// Atomically replaces a runtime status file with bounded UTF-8 text.
    pub fn write_runtime(&self, file: Runtime, text: &str) -> Result<(), StoreError> {
        if text.len() > MAX_RUNTIME_BYTES {
            return Err(StoreError::InvalidRuntime);
        }
        self.check_directory()?;
        let (target, temp) = file.names();
        self.write_private(temp, target, text.as_bytes())
    }

    /// Reads a runtime status file; a missing file means no report yet.
    pub fn read_runtime(&self, file: Runtime) -> Result<Option<String>, StoreError> {
        self.check_directory()?;
        let Some(handle) = self.private_file(file.names().0)? else {
            return Ok(None);
        };
        if handle.metadata()?.size() > MAX_RUNTIME_BYTES as u64 {
            return Err(StoreError::InvalidRuntime);
        }
        let mut text = String::new();
        handle
            .take(MAX_RUNTIME_BYTES as u64 + 1)
            .read_to_string(&mut text)
            .map_err(|_| StoreError::InvalidRuntime)?;
        if text.len() > MAX_RUNTIME_BYTES {
            return Err(StoreError::InvalidRuntime);
        }
        Ok(Some(text))
    }

    /// Checks the boot escape before any device open. An error must inhibit the operation.
    pub fn has_inactive_marker(&self) -> Result<bool, StoreError> {
        self.check_directory()?;
        Ok(self.private_file(STAY_INACTIVE)?.is_some())
    }

    /// Missing policy yields generation zero and no targets. Other failures must leave input inactive.
    pub fn load(&self, enrollments: &[VerifiedEnrollment<'_>]) -> Result<Policy, StoreError> {
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock_shared()?;
        self.load_locked(enrollments)
    }

    /// Reads recovery state without verifying enrollments, for the supervisor gate.
    /// The worker must still select devices only from [`Store::load`].
    pub fn recovery(&self) -> Result<Recovery, StoreError> {
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock_shared()?;
        Ok(self.load_unverified_locked()?.recovery())
    }

    /// Persists the restart latch at the current generation without verifying enrollments.
    pub fn latch_restart_limit(&self) -> Result<(), StoreError> {
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock()?;
        let mut policy = self.load_unverified_locked()?;
        if policy.recovery().latch == RecoveryLatch::RestartLimit {
            return Ok(());
        }
        let generation = policy
            .generation()
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        policy.set_generation(generation);
        policy.set_recovery(RecoveryOperation::LatchRestartLimit.apply(policy.recovery()));
        let canonical = policy.to_json();
        if canonical.len() > MAX_POLICY_BYTES {
            return Err(PolicyError::Oversized.into());
        }
        self.write_policy(&canonical, generation)
    }

    /// Reads durable enrollments. A missing file means none.
    pub fn enrollments(&self) -> Result<Vec<EnrollmentRecord>, StoreError> {
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock_shared()?;
        self.enrollments_locked()
    }

    /// Adds an enrollment made by [`VerifiedEnrollment::from_observation`]. Ordinary policy
    /// saves cannot create one. Adding an existing enrollment changes nothing.
    pub fn enroll(&self, enrollment: &VerifiedEnrollment<'_>) -> Result<(), StoreError> {
        #[cfg(feature = "reforge-passthrough")]
        if enrollment.evidence() == Evidence::XiaomiPixelosReforge20261111 {
            return Err(StoreError::InvalidEnrollments);
        }
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock()?;
        let mut records = self.enrollments_locked()?;
        let record = EnrollmentRecord {
            evidence: enrollment.evidence(),
            usb_serial: enrollment.serial().to_owned(),
        };
        if records.contains(&record) {
            return Ok(());
        }
        if records.len() >= MAX_ENROLLMENTS {
            return Err(StoreError::TooManyEnrollments);
        }
        records.push(record);
        let canonical = serde_json::to_vec(&Enrollments {
            schema_version: ENROLLMENT_SCHEMA,
            enrollments: records,
        })
        .map_err(|_| StoreError::InvalidEnrollments)?;
        self.write_private(ENROLLMENTS_TEMP, ENROLLMENTS, &canonical)?;
        self.directory.sync_all()?;
        Ok(())
    }

    fn enrollments_locked(&self) -> Result<Vec<EnrollmentRecord>, StoreError> {
        self.check_directory()?;
        let Some(file) = self.private_file(ENROLLMENTS)? else {
            return Ok(vec![]);
        };
        parse_enrollments(&read_bounded(file)?)
    }

    fn load_unverified_locked(&self) -> Result<Policy, StoreError> {
        self.check_directory()?;
        let Some(file) = self.private_file(POLICY)? else {
            return Ok(Policy::inactive());
        };
        let policy = Policy::parse_unverified(&read_bounded(file)?)?;
        if policy.generation() == 0 {
            return Err(StoreError::UnsafeStorage);
        }
        Ok(policy)
    }

    fn load_locked(&self, enrollments: &[VerifiedEnrollment<'_>]) -> Result<Policy, StoreError> {
        self.check_directory()?;
        let Some(file) = self.private_file(POLICY)? else {
            return Ok(Policy::inactive());
        };
        let policy = Policy::parse(&read_bounded(file)?, enrollments)?;
        if policy.generation() == 0 {
            return Err(StoreError::UnsafeStorage);
        }
        Ok(policy)
    }

    /// The request names the generation it edits. Success reports a durable save only.
    pub fn save(
        &self,
        bytes: &[u8],
        enrollments: &[VerifiedEnrollment<'_>],
    ) -> Result<Policy, StoreError> {
        let mut requested = Policy::parse(bytes, enrollments)?;

        // Reopen the directory to acquire a separate lock even for threads sharing this Store.
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock()?;
        let previous = self.load_locked(enrollments)?;
        let saved = previous.generation();
        if requested.generation() != saved {
            return Err(StoreError::StaleGeneration {
                requested: requested.generation(),
                saved,
            });
        }
        if requested.recovery() != previous.recovery() {
            return Err(StoreError::RecoveryChange);
        }

        let generation = saved
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        requested.set_generation(generation);
        let canonical = requested.to_json();
        if canonical.len() > MAX_POLICY_BYTES {
            return Err(PolicyError::Oversized.into());
        }

        self.write_policy(&canonical, generation)?;
        Ok(requested)
    }

    /// Applies a dedicated pause, boot-escape or restart-latch operation.
    ///
    /// The request names the generation it edits. Boot disable creates and synchronizes
    /// the pre-open marker before replacing the policy. Boot enable validates the marker,
    /// replaces the policy, then removes and synchronizes the marker.
    pub fn apply_recovery(
        &self,
        requested_generation: u64,
        operation: RecoveryOperation,
        enrollments: &[VerifiedEnrollment<'_>],
    ) -> Result<Policy, StoreError> {
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock()?;
        let mut policy = self.load_locked(enrollments)?;
        let saved = policy.generation();
        if requested_generation != saved {
            return Err(StoreError::StaleGeneration {
                requested: requested_generation,
                saved,
            });
        }
        let generation = saved
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        policy.set_generation(generation);
        policy.set_recovery(operation.apply(policy.recovery()));
        let canonical = policy.to_json();
        if canonical.len() > MAX_POLICY_BYTES {
            return Err(PolicyError::Oversized.into());
        }

        // A boot disable must publish the pre-open escape before the policy relies on it.
        // A boot enable validates an existing escape before changing the policy.
        match operation {
            RecoveryOperation::DisableNextBoot => self.set_inactive_marker(true)?,
            RecoveryOperation::EnableNextBoot => {
                self.has_inactive_marker()?;
            }
            _ => {}
        }

        self.write_policy(&canonical, generation)?;

        if operation == RecoveryOperation::EnableNextBoot {
            self.set_inactive_marker(false)
                .map_err(|source| StoreError::RecoveryIncomplete {
                    generation,
                    source: Box::new(source),
                })?;
        }
        Ok(policy)
    }

    fn write_policy(&self, canonical: &[u8], generation: u64) -> Result<(), StoreError> {
        self.write_private(TEMP, POLICY, canonical)?;
        self.directory
            .sync_all()
            .map_err(|source| StoreError::DurabilityUncertain { generation, source })
    }

    /// Atomically replaces `target` through `temp`. The caller synchronizes the directory.
    fn write_private(
        &self,
        temp_name: &CStr,
        target: &CStr,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        if self.private_file(temp_name)?.is_some() {
            unlink_at(&self.directory, temp_name)?;
        }
        let mut temp = open_at(
            &self.directory,
            temp_name,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        let replaced = (|| -> Result<(), StoreError> {
            self.check_private_file(&temp)?;
            temp.write_all(bytes)?;
            temp.sync_all()?;
            // SAFETY: Both names are fixed components under a live directory descriptor.
            let status = unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    temp_name.as_ptr(),
                    self.directory.as_raw_fd(),
                    target.as_ptr(),
                )
            };
            if status == -1 {
                return Err(io::Error::last_os_error().into());
            }
            Ok(())
        })();
        if let Err(error) = replaced {
            let _ = unlink_at(&self.directory, temp_name);
            return Err(error);
        }
        Ok(())
    }

    fn set_inactive_marker(&self, disabled: bool) -> Result<(), StoreError> {
        self.check_directory()?;
        if disabled {
            if self.private_file(STAY_INACTIVE)?.is_none() {
                let marker = open_at(
                    &self.directory,
                    STAY_INACTIVE,
                    libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
                )?;
                if let Err(error) = self.check_private_file(&marker) {
                    drop(marker);
                    let _ = unlink_at(&self.directory, STAY_INACTIVE);
                    return Err(error);
                }
                marker.sync_all()?;
            }
        } else if self.private_file(STAY_INACTIVE)?.is_some() {
            unlink_at(&self.directory, STAY_INACTIVE)?;
        } else {
            return Ok(());
        }
        self.directory.sync_all()?;
        Ok(())
    }

    fn private_file(&self, name: &CStr) -> Result<Option<File>, StoreError> {
        let mut metadata = MaybeUninit::<libc::stat>::uninit();
        // SAFETY: fstatat writes to valid storage and does not follow the final component.
        let status = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if status == -1 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(error.into())
            };
        }
        // SAFETY: Successful fstatat initialized the stat structure.
        let metadata = unsafe { metadata.assume_init() };
        if metadata.st_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(StoreError::UnsafeStorage);
        }
        let file = open_at(
            &self.directory,
            name,
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOCTTY,
        )?;
        self.check_private_file(&file)?;
        Ok(Some(file))
    }

    fn check_private_file(&self, file: &File) -> Result<(), StoreError> {
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != self.owner
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
        {
            return Err(StoreError::UnsafeStorage);
        }
        Ok(())
    }
}

fn read_bounded(file: File) -> Result<Vec<u8>, StoreError> {
    if file.metadata()?.size() > MAX_POLICY_BYTES as u64 {
        return Err(PolicyError::Oversized.into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_POLICY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn open_at(directory: &File, name: &CStr, flags: libc::c_int) -> io::Result<File> {
    // SAFETY: The descriptor and NUL-terminated name are valid during the call.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600 as libc::c_int,
        )
    };
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn unlink_at(directory: &File, name: &CStr) -> io::Result<()> {
    // SAFETY: The descriptor and NUL-terminated name are valid during the call.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
