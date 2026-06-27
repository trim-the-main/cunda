// The important list, firmware implements this:
pub const ENDPOINT_LIST: postcard_rpc::EndpointMap = crate::merge_endpoint_lists!(
    APPL_ENDPOINTS,
    crate::cunda_common::CUNDA_DEVICE_ENDPOINT,
    crate::cunda_common::v1::endpoints::CUNDA_SYS_ENDPOINTS,
);

use super::types::*;

endpoints_for_cunda! {
    list = APPL_ENDPOINTS;
    trait_name = DemoAppEndpoints;
    | EndpointTy              | RequestTy         | ResponseTy   | Path                        | Cfg |
    | ----------              | ---------         | ----------   | ----                        | --- |
    | GetApplSettings         | NoArg             | ApplSettings |"get_appl_settings"          |     |
    | SetApplSettings         | ApplSettings      | EmptyRes     |"set_appl_settings"          |     |
    | BlinkLedEndpoint        | u8                | EmptyRes     |"blink_led_n_times"          |     |
    | StartButtonEventsTopic  | NoArg             | EmptyRes     |"start_button_events_topic"  |     |
    | StopButtonEventsTopic   | NoArg             | EmptyRes     |"stop_button_events_topic"   |     |
    | EchoEndpoint            | EchoRequest       | EchoResponse |"echo"                       |     |
    | StartTestTopicBandwidth | NoArg             | EmptyRes     |"start_test_topic_bandwidth" |     |
    | StopTestTopicBandwidth  | NoArg             | EmptyRes     |"stop_test_topic_bandwidth"  |     |
    | TestBandwidth           | BandwidthTestData | EmptyRes     |"test_bandwidth"             |     |
}
