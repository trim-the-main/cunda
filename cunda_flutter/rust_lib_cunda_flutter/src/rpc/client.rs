use frb_prpc_juggle::client_interface::{
    Client, ClientEndpointInterface, ClientTopicInterface, FrbPostcardRpcError, TopicSink,
};
use postcard_rpc::Topic;
use serde::de::DeserializeOwned;

use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecoderDefmt, LogDecodingError},
    frb_generated::StreamSink,
    rpc::{LogDecoder, SysTopics},
};

pub struct FlutterClient {
    inner: Client,
    log_decoder: Option<LogDecoderDefmt>,
}

impl FlutterClient {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        Self {
            inner: Client::new(),
            log_decoder: None,
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn init(&mut self, sink: StreamSink<Vec<u8>>) {
        self.inner.init(Box::new(move |data| {
            log::trace!("Sending data from rust to flutter {:?}", data);
            if let Err(e) = sink.add(data) {
                todo!("Error sending data to flutter: {:?}", e);
            }
        }))
    }

    pub async fn rx_callback(
        &self,
        data: &[u8],
    ) -> Result<(), ::frb_prpc_juggle::client_interface::FrbPostcardRpcError> {
        self.inner.rx_callback(data).await
    }
}

impl LogDecoder for FlutterClient {
    fn init_log_decoder(&mut self, table_bytes: &[u8], loc_bytes: &[u8]) {
        let mut decoder = LogDecoderDefmt::load(table_bytes, Some(loc_bytes));
        // If for some reason we cannot parse the location bytes, carry on for now.
        if let Err(LogDecodingError::LocationDeserFailed) = decoder {
            decoder = LogDecoderDefmt::load(table_bytes, None);
        }
        self.log_decoder = decoder.ok();
    }

    #[flutter_rust_bridge::frb(sync)]
    fn decode_log(&self, bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError> {
        match self.log_decoder {
            Some(ref decoder) => decoder.defmt_decode(bytes),
            None => Err(LogDecodingError::NoTableData),
        }
    }
}

impl ClientEndpointInterface for FlutterClient {
    async fn call_rpc_endpoint<E: postcard_rpc::Endpoint>(
        &self,
        req: E::Request,
    ) -> Result<E::Response, FrbPostcardRpcError>
    where
        E::Request: serde::Serialize + postcard_schema::Schema + Send,
        E::Response: serde::de::DeserializeOwned,
    {
        self.inner
            .lock_for_endpoint_call()
            .await
            .call_rpc_endpoint::<E>(req)
            .await
    }
}

impl protocol::cunda_defaults::v1::endpoints::CundaSys for FlutterClient {}
impl protocol::devices::demo_esp32::v1::endpoints::DemoAppEndpoints for FlutterClient {}

impl ClientTopicInterface for FlutterClient {
    async fn subscribe<T: Topic>(&self, sink: Box<dyn TopicSink>) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        self.inner.subscribe::<T>(sink).await
    }

    async fn unsubscribe<T: Topic>(&self) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        self.inner.unsubscribe::<T>().await
    }
}

impl SysTopics for FlutterClient {}
protocol::devices::demo_esp32::v1::topics::define_topic_trait!(DemoAppTopics with StreamSink);
impl DemoAppTopics for FlutterClient {}
