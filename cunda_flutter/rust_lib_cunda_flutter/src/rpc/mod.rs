use frb_prpc_juggle::client_interface::TopicSink;
use serde::de::DeserializeOwned;

use crate::frb_generated::StreamSink;
pub use frb_prpc_juggle::client_interface::WireError;

pub mod client;
pub mod dummy_client;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
    #[cfg(target_os = "linux")]
    env_logger::builder().format_timestamp_micros().init();
}

#[flutter_rust_bridge::frb(mirror(WireError))]
pub struct _WireError {}

impl<T> TopicSink for StreamSink<T>
where
    T: DeserializeOwned,
    T: Send + Sync,
    T: crate::frb_generated::SseEncode,
{
    fn parse_and_add(&self, msg: &[u8]) -> Result<(), WireError> {
        match postcard::from_bytes::<T>(msg) {
            Ok(msg) => self.add(msg).map_err(|_err| WireError {}),
            Err(_e) => Err(WireError {}),
        }
    }
}

use protocol::{
    topics::{ButtonEvent, ButtonEvents, SysStatsTopic},
    v1::SysStats,
};
frb_prpc_juggle::topic_dispatcher_trait! {
    trait_name = TopicDispatcher;
    sink_type = StreamSink;
    version = "1";
    | TopicTy       | MessageTy
    | -------       | ---------
    | SysStatsTopic | SysStats
    | ButtonEvents  | ButtonEvent
}
