//! Key layout resolution and digest for an inventoried node.
//!
//! Android uses `Generic.kl` unless a device-specific layout or input
//! configuration exists. A node with such an override gets no digest, so it
//! cannot match evidence measured with `Generic.kl`.

use crate::inventory::Observation;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const GENERIC: &str = "system/usr/keylayout/Generic.kl";
const MAX_LAYOUT: u64 = 64 * 1024;
const LAYOUT_DIRS: [&str; 4] = [
    "odm/usr/keylayout",
    "vendor/usr/keylayout",
    "system/usr/keylayout",
    "data/system/devices/keylayout",
];
const IDC_DIRS: [&str; 4] = [
    "odm/usr/idc",
    "vendor/usr/idc",
    "system/usr/idc",
    "data/system/devices/idc",
];

/// Lowercase hex SHA-256 of `root/system/usr/keylayout/Generic.kl`, a regular file of at most 64 KiB.
pub fn generic_digest(root: &Path) -> io::Result<String> {
    let path = root.join(GENERIC);
    if !fs::metadata(&path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "layout is not a file",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(MAX_LAYOUT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_LAYOUT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "oversized layout",
        ));
    }
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// True if a device-specific `.kl` or `.idc` would replace `Generic.kl` for this node.
pub fn has_override(root: &Path, observed: &Observation) -> bool {
    let [_, vendor, product, version] = observed.input_id;
    let canonical: String = observed
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stems = [
        format!("Vendor_{vendor:04x}_Product_{product:04x}_Version_{version:04x}"),
        format!("Vendor_{vendor:04x}_Product_{product:04x}"),
        canonical,
    ];
    [(LAYOUT_DIRS, "kl"), (IDC_DIRS, "idc")]
        .iter()
        .flat_map(|(dirs, extension)| dirs.iter().map(move |dir| (dir, extension)))
        .any(|(dir, extension)| {
            stems.iter().any(|stem| {
                root.join(dir)
                    .join(format!("{stem}.{extension}"))
                    .symlink_metadata()
                    .is_ok()
            })
        })
}
