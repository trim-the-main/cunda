use frb_prpc_juggle::client_interface::TopicSink;
use serde::de::DeserializeOwned;

use crate::frb_generated::StreamSink;
pub use frb_prpc_juggle::client_interface::FrbPostcardRpcError;
pub use postcard_rpc::standard_icd::WireError;
pub use postcard_rpc::standard_icd::{FrameTooLong, FrameTooShort};

/// The mechanism that the rust side communicates with the outside world is
/// `FlutterWire`. When rust want to send bytes across the link, it hands the
/// data to flutter side using a stream (set by the `init` method). Flutter
/// listens to the stream and passes the bytes to the outside world. Flutter
/// side also listens for the data coming from the outside world and without
/// interpreting it, just passes it to rust side using `rx_callback`.
pub trait FlutterWire {
    /// Outbound data goes to the stream. Flutter side listens to this stream and
    /// sends the bytes over BLE
    #[flutter_rust_bridge::frb(sync)]
    fn init(&mut self, sink: StreamSink<Vec<u8>>);

    /// Incoming data read from BLE with flutter and we pass that data using a
    /// function call towards rust side
    #[allow(async_fn_in_trait)]
    async fn rx_callback(
        &self,
        data: &[u8],
    ) -> Result<(), ::frb_prpc_juggle::client_interface::FrbPostcardRpcError>;
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

protocol::cunda_common::v1::topics::define_topic_trait!(CundaSysT with StreamSink);
