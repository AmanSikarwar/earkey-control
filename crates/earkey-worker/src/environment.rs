//! Reads the environment fields that reviewed evidence pins.
//!
//! The kernel comes from `/proc/version`, not `uname`, which the reference
//! kernel spoofs. Any missing, empty, possibly truncated or malformed value is
//! an error, so an uncertain environment matches no evidence.

use earkey_policy::identity::{Environment, Evidence};
use std::io;

/// Android properties read, in [`Environment`] field order. `ro.custom.version`
/// is the PixelOS ROM property identified in the 10 November revalidation.
pub const PROPERTIES: [&str; 5] = [
    "ro.product.model",
    "ro.product.device",
    "ro.custom.version",
    "ro.build.id",
    "ro.build.version.incremental",
];
const SDK: &str = "ro.build.version.sdk";
/// Bionic truncates longer values to this many bytes, so a value this long is ambiguous.
const TRUNCATED: usize = 91;
#[cfg(target_os = "android")]
const MAX_PROC_VERSION: u64 = 1024;

/// Evidence that may be selected, by environment.
pub const EVIDENCE: [Evidence; 3] = [
    Evidence::XiaomiPixelos20261107,
    Evidence::XiaomiPixelosReforge20261110,
    Evidence::XiaomiPixelosReforge20261111,
];

/// The reviewed evidence whose pinned environment matches `observed`.
pub fn evidence_for(observed: &Observed) -> Option<Evidence> {
    let environment = observed.as_environment();
    EVIDENCE
        .into_iter()
        .find(|evidence| evidence.matches_environment(&environment))
}

/// An owned environment observation.
#[derive(Debug, Eq, PartialEq)]
pub struct Observed {
    pub model: String,
    pub device: String,
    pub api: u32,
    pub rom: String,
    pub build_id: String,
    pub incremental: String,
    pub kernel_release: String,
    pub kernel_build: String,
}

impl Observed {
    pub fn as_environment(&self) -> Environment<'_> {
        Environment {
            model: &self.model,
            device: &self.device,
            api: self.api,
            rom: &self.rom,
            build_id: &self.build_id,
            incremental: &self.incremental,
            kernel_release: &self.kernel_release,
            kernel_build: &self.kernel_build,
        }
    }
}

/// Builds an observation from a property source and the text of `/proc/version`.
pub fn parse(
    property: impl Fn(&str) -> Option<String>,
    proc_version: &str,
) -> io::Result<Observed> {
    let value = |name: &str| {
        property(name)
            .filter(|value| valid(value) && value.len() < TRUNCATED)
            .ok_or_else(|| invalid(name))
    };
    let [model, device, rom, build_id, incremental] = PROPERTIES.map(value);
    let api = value(SDK)?.parse().map_err(|_| invalid(SDK))?;
    let (kernel_release, kernel_build) = kernel(proc_version)?;
    Ok(Observed {
        model: model?,
        device: device?,
        api,
        rom: rom?,
        build_id: build_id?,
        incremental: incremental?,
        kernel_release,
        kernel_build,
    })
}

/// Splits `Linux version <release> (<builder>) (<compiler>) <build>`.
fn kernel(text: &str) -> io::Result<(String, String)> {
    let line = text.strip_suffix('\n').unwrap_or(text);
    let rest = line
        .strip_prefix("Linux version ")
        .ok_or_else(|| invalid("/proc/version"))?;
    let release = rest.split(' ').next().unwrap_or_default();
    let build = rest
        .rfind(") #")
        .map(|index| &rest[index + 2..])
        .unwrap_or_default();
    if !valid(release)
        || release.contains(char::is_whitespace)
        || !valid(build)
        || !build[1..].starts_with(|c: char| c.is_ascii_digit())
    {
        return Err(invalid("/proc/version"));
    }
    Ok((release.to_owned(), build.to_owned()))
}

fn valid(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("environment: {what}"))
}

/// Reads the live environment.
#[cfg(target_os = "android")]
pub fn read() -> io::Result<Observed> {
    use std::io::Read;
    let mut text = String::new();
    std::fs::File::open("/proc/version")?
        .take(MAX_PROC_VERSION + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_PROC_VERSION {
        return Err(invalid("/proc/version"));
    }
    parse(system_property, &text)
}

#[cfg(target_os = "android")]
fn system_property(name: &str) -> Option<String> {
    let name = std::ffi::CString::new(name).ok()?;
    let mut buffer = [0u8; libc::PROP_VALUE_MAX as usize];
    // SAFETY: The name is NUL-terminated and the buffer holds PROP_VALUE_MAX bytes.
    let length = unsafe { libc::__system_property_get(name.as_ptr(), buffer.as_mut_ptr().cast()) };
    let length = usize::try_from(length).ok().filter(|&length| length > 0)?;
    String::from_utf8(buffer.get(..length)?.to_vec()).ok()
}
