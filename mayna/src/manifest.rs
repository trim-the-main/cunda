use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Metadata extracted from the `.mayna_meta` ELF section.
/// This is the JSON that the `mayna_meta!` macro embeds in the ELF.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirmwareInfo {
    pub device_type: String,
    pub firmware_version: String,
    pub protocol_version: u32,
}

/// A single component (file) within a `.mayna` archive.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Component {
    pub file: String,
    pub size: u64,
    pub sha256: String,
}

/// The manifest.json inside a `.mayna` archive.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageManifest {
    pub format_version: u32,
    pub device_type: String,
    pub firmware_version: String,
    pub protocol_version: u32,
    pub min_firmware_version: String,
    pub publish_date: String,
    pub components: BTreeMap<String, Component>,
}

impl PackageManifest {
    pub const FORMAT_VERSION: u32 = 1;
}
