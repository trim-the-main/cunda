#[cfg(feature = "flutter")]
use frb_prpc_juggle::{endpoint_handler_trait_for_flutter, endpoints_for_flutter as endpoints};
#[cfg(not(feature = "flutter"))]
use postcard_rpc::endpoints;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub type VersionString = ProtocolStringType!(capacity: 16);
endpoints! {
    list = ENDPOINT_LIST;
    omit_std = true;
    // System Endpoints
    | EndpointTy          | RequestTy   | ResponseTy             | Path                    |
    | ----------          | ---------   | ----------             | ----                    |
    | GetFirmwareVersion  | NoArg       | VersionString          | "get_firmware_version"  |
    | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
    | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
    | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
    | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
    | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
    // Application Endpoints
    | GetApplSettings      | NoArg        | ApplSettings         | "get_appl_settings"  |
    | SetApplSettings      | ApplSettings | EmptyRes             | "set_appl_settings"  |
    | BlinkLedEndpoint     | u8           | EmptyRes             | "blink_led_n_times"  |
    | StartButtonEventsTopic  | NoArg       | EmptyRes               | "start_button_events_topic" |
    | StopButtonEventsTopic   | NoArg       | EmptyRes               | "stop_button_events_topic"  |
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

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
}
