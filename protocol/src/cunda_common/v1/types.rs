use crate::cunda_common::VersionString;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub use crate::cunda_common::{DeviceId, GitRevSha};
pub use crate::types::{EmptyRes, NoArg};

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct OtaMData {
    pub size: u32,
    pub hash_sha256: [u8; 32],
    pub version: VersionString,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub enum OtaResult {
    TransferReady,
    TransferComplete,
    Restarting,
    StorageError,
    VerificationError,
    NotSupported,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct OtaBytes {
    pub offset: u32,
    pub data: ProtocolVecType!(u8, 4096),
}

/// LOGS TOPIC
#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct LogMessage {
    pub defmt_bytes: ProtocolVecType!(u8, 1024),
}

#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct Percent(pub u8);

#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct CpuUsage {
    pub core0: Percent,
    pub core1: Percent,
}
#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct MemoryUsage {
    pub used: u32,
    pub total: u32,
}

#[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
pub struct SysStats {
    pub cpu_usage: CpuUsage,
    pub memory_usage: MemoryUsage,
    pub uptime: u32,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct SysSettings {
    // TODO: move the heapless constants to somewhere
    pub wifi_ssid: ProtocolStringType!(32),
    pub wifi_password: ProtocolStringType!(63),
    pub ble_device_name: ProtocolStringType!(20),
}
