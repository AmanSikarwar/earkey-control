//! Input worker. This revision contains read-only observation and the acquisition check.

#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod check;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod device;
pub mod environment;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod forward;
pub mod inventory;
pub mod layout;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod proxy;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod proxy_check;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod session;
