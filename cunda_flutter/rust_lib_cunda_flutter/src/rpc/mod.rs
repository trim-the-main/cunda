use frb_prpc_juggle::client_interface::TopicSink;
use protocol::devices::demo_esp32::v1::{topics::*, types::*};
use serde::de::DeserializeOwned;

use crate::defmt_log_translation::{DefmtLogEntry, LogDecodingError};
use crate::frb_generated::StreamSink;
pub use frb_prpc_juggle::client_interface::FrbPostcardRpcError;
pub use postcard_rpc::standard_icd::WireError;
pub use postcard_rpc::standard_icd::{FrameTooLong, FrameTooShort};
pub mod client;
pub mod dummy_client;

/// Interface for defmt log decoding.
pub trait LogDecoder {
    /// Load defmt table and location data to prepare for decoding.
    fn init_log_decoder(&mut self, table_bytes: &[u8], loc_bytes: &[u8]);

    /// Decode a raw defmt byte stream into structured log entries.
    #[flutter_rust_bridge::frb(sync)]
    fn decode_log(&self, bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError>;
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
    #[cfg(target_os = "linux")]
    env_logger::builder().format_timestamp_micros().init();
}

#[flutter_rust_bridge::frb(mirror(FrameTooLong))]
pub struct _FrameTooLong {
    /// The length of the too-long frame
    pub len: u32,
    /// The maximum frame length supported
    pub max: u32,
}

#[flutter_rust_bridge::frb(mirror(FrameTooShort))]
pub struct _FrameTooShort {
    pub len: u32,
}

#[flutter_rust_bridge::frb(mirror(WireError))]
pub enum _WireError {
    /// The frame exceeded the buffering capabilities of the server
    FrameTooLong(FrameTooLong),
    /// The frame was shorter than the minimum frame size and was rejected
    FrameTooShort(FrameTooShort),
    /// Deserialization of a message failed
    DeserFailed,
    /// Serialization of a message failed, usually due to a lack of space to
    /// buffer the serialized form
    SerFailed,
    /// The key associated with this request was unknown
    UnknownKey,
    /// The server was unable to spawn the associated handler, typically due
    /// to an exhaustion of resources
    FailedToSpawn,
    /// The provided key is below the minimum key size calculated to avoid hash
    /// collisions, and was rejected to avoid potential misunderstanding
    KeyTooSmall,
}

#[flutter_rust_bridge::frb(mirror(FrbPostcardRpcError))]
pub enum _FrbPostcardRpError {
    InternalError,
    OnlyOneDartStreamAllowed,
    DeserializationError,
    AlreadySubscribedtoTopic,
    NotSubscribedToTopic,
    RpcError(WireError),
}

impl<T> TopicSink for StreamSink<T>
where
    T: DeserializeOwned,
    T: Send + Sync,
    T: crate::frb_generated::SseEncode,
{
    fn parse_and_add(&self, msg: &[u8]) -> Result<(), FrbPostcardRpcError> {
        match postcard::from_bytes::<T>(msg) {
            Ok(msg) => self.add(msg).map_err(|err| {
                log::error!(
                    "Error trying to send topic frame to from rust to dart {}",
                    err
                );
                FrbPostcardRpcError::InternalError
            }),
            Err(err) => {
                log::error!("Deserialization error {} on topic message", err);
                Err(FrbPostcardRpcError::DeserializationError)
            }
        }
    }
}

frb_prpc_juggle::topic_dispatcher_trait! {
    trait_name = TopicDispatcher;
    sink_type = StreamSink;
    | TopicTy       | MessageTy
    | -------       | ---------
    | SysStatsTopic | SysStats
    | SysLogsTopic  | LogMessage
    | ButtonEvents  | ButtonEvent
    | BandwidthTestTopic | BandwidthTestTopicData
}
