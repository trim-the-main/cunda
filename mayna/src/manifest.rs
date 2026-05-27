use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

/// Metadata extracted from the `.mayna_meta` ELF section.
/// This is the JSON that the `mayna_meta!` macro embeds in the ELF.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirmwareInfo {
    pub device_type: String,
    pub firmware_version: String,
    pub protocol_version: u32,
    #[serde(default)]
    pub git_hash: Option<String>,
}

/// Known component keys within a `.mayna` archive.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    Firmware,
    DefmtTable,
    DefmtLocations,
    PartitionTable,
    Changelog,
    Bootloader,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_hash: Option<String>,
    pub min_firmware_version: String,
    pub publish_date: String,
    #[serde(deserialize_with = "deserialize_components")]
    pub components: BTreeMap<ComponentType, Component>,
}

impl ComponentType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "firmware" => Some(Self::Firmware),
            "defmt_table" => Some(Self::DefmtTable),
            "defmt_locations" => Some(Self::DefmtLocations),
            "partition_table" => Some(Self::PartitionTable),
            "changelog" => Some(Self::Changelog),
            "bootloader" => Some(Self::Bootloader),
            _ => None,
        }
    }
}

impl PackageManifest {
    pub const FORMAT_VERSION: u32 = 1;
}

fn deserialize_components<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<ComponentType, Component>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: BTreeMap<String, Component> = BTreeMap::deserialize(deserializer)?;
    let mut result = BTreeMap::new();
    for (key_str, component) in raw {
        match ComponentType::from_str(&key_str) {
            Some(key) => {
                result.insert(key, component);
            }
            None => {
                eprintln!("Warning: unknown component key '{key_str}', skipping");
            }
        }
    }
    Ok(result)
}
