use crate::identity::VerifiedEnrollment;
use crate::policy::{MAX_POLICY_BYTES, Policy, PolicyError};
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

const POLICY: &CStr = c"policy.json";
const TEMP: &CStr = c"policy.tmp";
const DIRECTORY: libc::c_int = libc::O_RDONLY | libc::O_DIRECTORY;

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    UnsafeStorage,
    Policy(PolicyError),
    StaleGeneration { requested: u64, saved: u64 },
    GenerationExhausted,
    RecoveryChange,
    DurabilityUncertain { generation: u64, source: io::Error },
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

    /// Missing policy yields generation zero and no targets. Other failures must leave input inactive.
    pub fn load(&self, enrollments: &[VerifiedEnrollment<'_>]) -> Result<Policy, StoreError> {
        let lock = open_at(&self.directory, c".", DIRECTORY)?;
        lock.lock_shared()?;
        self.load_locked(enrollments)
    }

    fn load_locked(&self, enrollments: &[VerifiedEnrollment<'_>]) -> Result<Policy, StoreError> {
        self.check_directory()?;
        let Some(file) = self.private_file(POLICY)? else {
            return Ok(Policy::inactive());
        };
        if file.metadata()?.size() > MAX_POLICY_BYTES as u64 {
            return Err(PolicyError::Oversized.into());
        }

        let mut bytes = Vec::new();
        file.take(MAX_POLICY_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let policy = Policy::parse(&bytes, enrollments)?;
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
            temp.write_all(&canonical)?;
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
            .map_err(|source| StoreError::DurabilityUncertain { generation, source })?;
        Ok(requested)
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
