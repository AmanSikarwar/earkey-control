//! Bounded process-group supervision for the experimental observer.
//! This library does not implement tracing or expose a device command.
//! Spawn and supervisor failure still require an independent watchdog; this
//! component is not eligible for device use on its own.

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub use linux::{Outcome, run};
