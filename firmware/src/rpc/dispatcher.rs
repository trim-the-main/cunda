use postcard_rpc::define_dispatch;
use protocol::devices::demo_esp32::v1::{endpoints::*, topics};

use postcard_rpc::server::impls::embedded_io_async_v0_6::dispatch_impl::{WireSpawnImpl, spawn_fn};

// importing these handlers to the namespace because the `define_dispatch` macro does not match
// fully qualified paths for handler functions, it expects identifiers
use crate::rpc::app_handlers::*;
use crate::rpc::sys_handlers::*;
use protocol::cunda_common::GetDeviceId;
use protocol::cunda_common::v1::endpoints::*;

define_dispatch! {
    app: BleDispatcher;
    spawn_fn: spawn_fn;
    tx_impl: crate::ble::BleWireTxImpl;
    spawn_impl: WireSpawnImpl;
    context: crate::rpc::context::DispatchContext;

    endpoints: {
        list: ENDPOINT_LIST;
        | EndpointTy          | kind                | handler               |
        | ----------          | ----                | -------               |
        | GetDeviceId         | blocking            | get_device_id         |
        | GetSysSettings      | async               | get_sys_settings      |
        | SetSysSettings      | async               | set_sys_settings      |
        | PingEndpoint        | blocking            | sys_ping              |
        | StartSysStatsTopic  | spawn               | start_sys_stats_topic |
        | StopSysStatsTopic   | blocking            | stop_sys_stats_topic  |
        | StartSysLogsTopic   | spawn               | start_sys_logs_topic  |
        | StopSysLogsTopic    | blocking            | stop_sys_logs_topic   |
        | GetMtu              | async               | get_mtu               |
        | PrepareOta          | async | ota_prepare          |
        | TransferOtaBytes    | async | ota_transfer_bytes   |
        | FinalizeOta         | async | ota_finalize         |
        | ApproveFirmware     | async | ota_approve_firmware |
        | FactoryReset        | async | ota_factory_reset    |
        // Application Endpoints
        | GetApplSettings         | async             | get_appl_settings  |
        | SetApplSettings         | async             | set_appl_settings  |
        | BlinkLedEndpoint        | async             | blink_led_n_times  |
        | StartButtonEventsTopic  | spawn             | start_button_events_topic |
        | StopButtonEventsTopic   | async             | stop_button_events_topic  |
        | EchoEndpoint            | blocking          | echo                      |
        | StartTestTopicBandwidth | spawn             | start_bandwidth_test_topic           |
        | StopTestTopicBandwidth  | async             | stop_bandwidth_test_topic            |
        | TestBandwidth           | blocking          | do_nothing                           |
    };
    topics_in: {
        list: topics::EMPTY_TOPICS;

        | TopicTy                   | kind      | handler                       |
        | ----------                | ----      | -------                       |
    };
    topics_out: {
        list: topics::TOPICS;
    };
}
