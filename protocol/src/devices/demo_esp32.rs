// Demo device for cunda framework using esp32
//

pub const DEVICE_TYPE: &str = "demo-esp32";

pub mod v1 {
    pub const RPC_PROTOCOL_VERSION: u32 = 1;

    pub mod types {
        pub use crate::types::cunda_defaults::*;
        use postcard_schema::Schema;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Schema, Debug, Clone)]
        pub struct ApplSettings {
            pub led_blink_duration_ms: u32,
        }

        #[derive(Serialize, Deserialize, Schema, Debug, Clone)]
        pub struct EchoRequest {
            pub inner: ProtocolStringType!(capacity: 512),
        }

        #[derive(Serialize, Deserialize, Schema, Debug, Clone)]
        pub struct EchoResponse {
            pub inner: ProtocolStringType!(capacity: 512),
        }

        #[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
        pub struct BandwidthTestData {
            pub data: ProtocolVecType!(u8, 2048),
        }

        #[derive(Serialize, Deserialize, Schema, Debug, Copy, Clone, Default)]
        pub struct ButtonEvent {
            pub press_time_in_ms: u32,
        }

        const BW_TEST_SIZE: usize = 1900;
        #[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
        pub struct BandwidthTestTopicData {
            pub nums: ProtocolVecType!(u8, BW_TEST_SIZE),
        }

        impl BandwidthTestTopicData {
            pub fn new(first: u8) -> Self {
                Self {
                    nums: (0..BW_TEST_SIZE)
                        .map(|i| first.wrapping_add(i as u8))
                        .collect(),
                }
            }
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
    }

    pub mod endpoints {
        use crate::devices::demo_esp32::v1::types::*;

        endpoints_with_cunda_defaults! {
            list = ENDPOINT_LIST;
            appl_trait_name = DemoAppEndpoints;
            | EndpointTy              | RequestTy         | ResponseTy   | Path                        | Cfg |
            | ----------              | ---------         | ----------   | ----                        | --- |
            | GetApplSettings         | NoArg             | ApplSettings |"get_appl_settings"          |
            | SetApplSettings         | ApplSettings      | EmptyRes     |"set_appl_settings"          |
            | BlinkLedEndpoint        | u8                | EmptyRes     |"blink_led_n_times"          |
            | StartButtonEventsTopic  | NoArg             | EmptyRes     |"start_button_events_topic"  |
            | StopButtonEventsTopic   | NoArg             | EmptyRes     |"stop_button_events_topic"   |
            | EchoEndpoint            | EchoRequest       | EchoResponse |"echo"                       |
            | StartTestTopicBandwidth | NoArg             | EmptyRes     |"start_test_topic_bandwidth" |
            | StopTestTopicBandwidth  | NoArg             | EmptyRes     |"stop_test_topic_bandwidth"  |
            | TestBandwidth           | BandwidthTestData | EmptyRes     |"test_bandwidth"             |
        }
    }

    pub mod topics {
        use crate::devices::demo_esp32::v1::types::*;

        topics_with_cunda_defaults! {
            list = TOPICS;
            | TopicTy       | MessageTy     | Path               | Cfg |
            | -------       | ---------     | ----               | --- |
            | ButtonEvents       | ButtonEvent           | "button_events"        |     |
            | BandwidthTestTopic | BandwidthTestTopicData| "bandwidth_test_topic" |     |
        }

        #[cfg(not(feature = "flutter"))]
        postcard_rpc::topics! {
           list = EMPTY_TOPICS;
           direction = postcard_rpc::TopicDirection::ToServer;
           | TopicTy        | MessageTy     | Path              |
           | -------        | ---------     | ----              |
        }
    }
}
