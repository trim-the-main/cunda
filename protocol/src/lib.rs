#![cfg_attr(all(not(feature = "flutter"), not(test)), no_std)]

#[macro_use]
pub mod type_helpers;

#[macro_use]
pub mod cunda_macros;

#[macro_use]
pub mod mayna_macros;

pub mod devices;
pub mod merge_utils;
pub mod types;

pub mod cunda_defaults {
    pub use crate::types::*;
    #[cfg(feature = "flutter")]
    pub use frb_prpc_juggle::client_interface::FrbPostcardRpcError;

    pub mod v1 {
        pub mod endpoints {
            pub use super::types::{DeviceId, OtaBytes, OtaMData, OtaResult, SysSettings};
            pub use crate::types::*;

            endpoints_for_cunda! {
                list = CUNDA_DEFAULT_ENDPOINTS;
                trait_name = CundaSys;

                | EndpointTy          | RequestTy   | ResponseTy             | Path                    |
                | ----------          | ---------   | ----------             | ----                    |
                // SYSTEM
                | GetDeviceId         | NoArg       | DeviceId               | "get_device_id"         |
                | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
                | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
                | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
                | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
                | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
                | GetMtu              | NoArg       | u16                    | "get_mtu"               |
                | StartSysLogsTopic   | NoArg       | EmptyRes               | "start_sys_logs_topic"  |
                | StopSysLogsTopic    | NoArg       | EmptyRes               | "stop_sys_logs_topic"   |
                // OTA
                | PrepareOta          | OtaMData    | OtaResult              | "prepare_ota"              |
                | TransferOtaBytes    | OtaBytes    | OtaResult              | "transfer_ota_bytes"       |
                | FinalizeOta         | NoArg       | OtaResult              | "finalize_ota"             |
                | ApproveFirmware     | NoArg       | OtaResult              | "approve_firmware_version" |
                | FactoryReset        | NoArg       | OtaResult              | "factory_reset"            |
            }
        }

        pub mod topics {
            pub use super::types::{LogMessage, SysStats};

            topics_for_cunda! {
                list = CUNDA_DEFAULT_TOPICS;
                trait_name = SysTopics;
                path = cunda_defaults::v1::topics;
                | TopicTy       | MessageTy     | Path               | Cfg |
                | -------       | ---------     | ----               | --- |
                | SysStatsTopic | SysStats      | "sys_stats_stream" |     |
                | SysLogsTopic  | LogMessage    | "sys_logs_stream"  |     |
            }
        }

        pub mod types {
            pub use crate::types::*;
            use postcard_schema::Schema;
            use serde::{Deserialize, Serialize};

            use crate::type_helpers::hex_nibble;

            pub type VersionString = ProtocolStringType!(capacity: 16);
            #[derive(
                Serialize, Deserialize, Schema, Debug, Clone, Copy, PartialEq, Eq, Default,
            )]
            pub struct GitRevSha(pub [u8; 20]);
            #[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
            pub struct DeviceId {
                pub device_type: ProtocolStringType!(capacity: 16),
                pub hardware_revision: u32,
                pub serial_number: u32,
                pub firmware_version: VersionString,
                pub protocol_version: u32,
                pub git_hash: Option<GitRevSha>,
            }

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

            impl GitRevSha {
                pub const fn from_hex_str(hex: &str) -> Option<Self> {
                    let bytes = hex.as_bytes();
                    if bytes.len() != 40 {
                        return None;
                    }
                    // "git hash must be 40 hex characters");

                    let mut result = [0u8; 20];
                    let mut i = 0;
                    while i < 20 {
                        result[i] = hex_nibble(bytes[i * 2]) * 16 + hex_nibble(bytes[i * 2 + 1]);
                        i += 1;
                    }
                    Some(Self(result))
                }
            }

            impl core::fmt::Display for GitRevSha {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    for byte in &self.0 {
                        write!(f, "{:02x}", byte)?;
                    }
                    Ok(())
                }
            }

            impl DeviceId {
                pub fn new(
                    device_type: &str,
                    hardware_revision: u32,
                    serial_number: u32,
                    firmware_version: &str,
                    protocol_version: u32,
                    git_hash: Option<GitRevSha>,
                ) -> Self {
                    use core::str::FromStr;
                    Self {
                        device_type: FromStr::from_str(device_type).unwrap(),
                        hardware_revision,
                        serial_number,
                        firmware_version: FromStr::from_str(firmware_version).unwrap(),
                        protocol_version,
                        git_hash,
                    }
                }
            }
        }
    }
}
