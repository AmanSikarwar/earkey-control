use crate::identity::VerifiedEnrollment;
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
        if self.private_file(TEMP)?.is_some() {
            unlink_at(&self.directory, TEMP)?;
        }
        let mut temp = open_at(
            &self.directory,
            TEMP,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        let replaced = (|| -> Result<(), StoreError> {
            self.check_private_file(&temp)?;
            temp.write_all(canonical)?;
            temp.sync_all()?;
            // SAFETY: Both names are fixed components under a live directory descriptor.
            let status = unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    TEMP.as_ptr(),
                    self.directory.as_raw_fd(),
                    POLICY.as_ptr(),
                )
            };
            if status == -1 {
                return Err(io::Error::last_os_error().into());
            }
            Ok(())
        })();
        if let Err(error) = replaced {
            let _ = unlink_at(&self.directory, TEMP);
            return Err(error);
        }
        self.directory
            .sync_all()
            .map_err(|source| StoreError::DurabilityUncertain { generation, source })
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
