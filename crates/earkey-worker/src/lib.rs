//! Input worker. This revision contains read-only inventory and environment observation.

#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod device;
pub mod environment;
pub mod inventory;
