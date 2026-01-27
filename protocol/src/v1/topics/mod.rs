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
    // Application topics
    | ButtonEvents  | ButtonEvent   | "button_events"    |     |
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
