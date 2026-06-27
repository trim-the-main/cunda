
pub use crate::cunda_common::v1::types::*;
pub use crate::types::*;
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
