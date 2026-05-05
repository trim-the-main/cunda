// Definitions for Rpc communication

use crate::v1::SysStats;
use postcard_rpc::TopicDirection;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "flutter")]
use frb_prpc_juggle::{make_sure_direction_is_to_client, topics_for_flutter as topics};
#[cfg(not(feature = "flutter"))]
use postcard_rpc::topics;

topics! {
    list = TOPICS;
    direction = TopicDirection::ToClient;
    | TopicTy       | MessageTy     | Path               | Cfg |
    | -------       | ---------     | ----               | --- |
    // System topics
    | SysStatsTopic | SysStats      | "sys_stats_stream" |     |
    | SysLogsTopic  | LogMessage    | "sys_logs_stream"  |     |
    // Application topics
    | ButtonEvents       | ButtonEvent           | "button_events"        |     |
    | BandwidthTestTopic | BandwidthTestTopicData| "bandwidth_test_topic" |     |
}
#[cfg(not(feature = "flutter"))]
topics! {
   list = EMPTY_TOPICS;
   direction = TopicDirection::ToServer;
   | TopicTy        | MessageTy     | Path              |
   | -------        | ---------     | ----              |
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

#[derive(Serialize, Deserialize, Schema, Debug, Clone, Default)]
pub struct LogMessage {
    pub defmt_bytes: ProtocolVecType!(u8, 1024),
}
