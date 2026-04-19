#[cfg(feature = "flutter")]
use frb_prpc_juggle::{endpoint_handler_trait_for_flutter, endpoints_for_flutter as endpoints};
#[cfg(not(feature = "flutter"))]
use postcard_rpc::endpoints;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub type VersionString = ProtocolStringType!(capacity: 16);
endpoints! {
    list = ENDPOINT_LIST;
    // System Endpoints
    | EndpointTy          | RequestTy   | ResponseTy             | Path                    |
    | ----------          | ---------   | ----------             | ----                    |
    | GetFirmwareVersion  | NoArg       | VersionString          | "get_firmware_version"  |
    | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
    | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
    | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
    | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
    | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
    | GetMtu              | NoArg       | u16                    | "get_mtu"               |
    // OTA related endpoints
    | PrepareOta          | OtaMData    | OtaResult              | "prepare_ota"              |
    | TransferOtaBytes    | OtaBytes    | OtaResult              | "transfer_ota_bytes"       |
    | FinalizeOta         | NoArg       | OtaResult              | "finalize_ota"             |
    | ApproveFirmware     | NoArg       | OtaResult              | "approve_firmware_version" |
    | FactoryReset        | NoArg       | OtaResult              | "factory_reset"            |
    // Application Endpoints
    | GetApplSettings         | NoArg             | ApplSettings       | "get_appl_settings"         |
    | SetApplSettings         | ApplSettings      | EmptyRes           | "set_appl_settings"         |
    | BlinkLedEndpoint        | u8                | EmptyRes           | "blink_led_n_times"         |
    | StartButtonEventsTopic  | NoArg             | EmptyRes           | "start_button_events_topic" |
    | StopButtonEventsTopic   | NoArg             | EmptyRes           | "stop_button_events_topic"  |
    | EchoEndpoint            | EchoRequest       | EchoResponse       | "echo"                      |
    | StartTestTopicBandwidth | NoArg             | EmptyRes           | "start_test_topic_bandwidth"|
    | StopTestTopicBandwidth  | NoArg             | EmptyRes           | "stop_test_topic_bandwidth" |
    | TestBandwidth           | BandwidthTestData           | EmptyRes           | "test_bandwidth"            |
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct NoArg {}

impl From<()> for NoArg {
    fn from(_value: ()) -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct EmptyRes {}

impl From<()> for EmptyRes {
    fn from(_value: ()) -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub enum PError {
    SpawnError,
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct SysSettings {
    // TODO: move the heapless constants to somewhere
    pub wifi_ssid: ProtocolStringType!(32),
    pub wifi_password: ProtocolStringType!(63),
    pub ble_device_name: ProtocolStringType!(20),
}

#[cfg(feature = "flutter")]
use std::string::String;

#[cfg(not(feature = "flutter"))]
use heapless::String;

impl SysSettings {
    pub const fn new() -> Self {
        Self {
            wifi_ssid: String::new(),
            wifi_password: String::new(),
            ble_device_name: String::new(),
        }
    }
}
#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub struct ApplSettings {
    pub led_blink_duration_ms: u32,
}

impl ApplSettings {
    pub const fn new() -> Self {
        Self {
            led_blink_duration_ms: 100,
        }
    }
}

impl Default for ApplSettings {
    fn default() -> Self {
        Self {
            led_blink_duration_ms: 100,
        }
    }
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub struct EchoRequest {
    pub inner: ProtocolStringType!(capacity: 512),
}

#[derive(Serialize, Deserialize, Schema, Debug, Clone)]
pub struct EchoResponse {
    pub inner: ProtocolStringType!(capacity: 512),
}

////////////////////////
/// OTA
///////////////////////
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

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct BandwidthTestData {
    pub data: ProtocolVecType!(u8, 2048),
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
}
