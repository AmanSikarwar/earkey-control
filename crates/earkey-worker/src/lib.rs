//! Input worker. This revision contains read-only observation and the acquisition check.

#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod check;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod device;
pub mod environment;
pub mod inventory;
