pub use super::types::{OtaBytes, OtaMData, OtaResult, SysSettings};
pub use crate::types::{EmptyRes, NoArg};

endpoints_for_cunda! {
    list = CUNDA_SYS_ENDPOINTS;
    trait_name = CundaSysE;

    | EndpointTy          | RequestTy   | ResponseTy             | Path                    |
    | ----------          | ---------   | ----------             | ----                    |
    // SYSTEM
    | GetSysSettings      | NoArg       | SysSettings            | "get_sys_settings"      |
    | SetSysSettings      | SysSettings | EmptyRes               | "set_sys_settings"      |
    | StartSysStatsTopic  | NoArg       | EmptyRes               | "start_sys_stats_topic" |
    | StopSysStatsTopic   | NoArg       | EmptyRes               | "stop_sys_stats_topic"  |
    | StartSysLogsTopic   | NoArg       | EmptyRes               | "start_sys_logs_topic"  |
    | StopSysLogsTopic    | NoArg       | EmptyRes               | "stop_sys_logs_topic"   |
    | GetMtu              | NoArg       | u16                    | "get_mtu"               |
    | PingEndpoint        | NoArg       | EmptyRes               | "sys_ping"              |
    // OTA
    | PrepareOta          | OtaMData    | OtaResult              | "prepare_ota"              |
    | TransferOtaBytes    | OtaBytes    | OtaResult              | "transfer_ota_bytes"       |
    | FinalizeOta         | NoArg       | OtaResult              | "finalize_ota"             |
    | ApproveFirmware     | NoArg       | OtaResult              | "approve_firmware_version" |
    | FactoryReset        | NoArg       | OtaResult              | "factory_reset"            |
}
