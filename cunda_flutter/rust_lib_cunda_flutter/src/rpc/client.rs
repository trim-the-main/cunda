use frb_prpc_juggle::client_interface::{
    Client, ClientEndpointInterface, ClientTopicInterface, TopicSink, WireError,
};
use postcard_rpc::Topic;
use serde::de::DeserializeOwned;

use crate::{frb_generated::StreamSink, rpc::TopicDispatcher};

pub struct FlutterClient {
    inner: Client,
}

impl FlutterClient {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        Self {
            inner: Client::new(),
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn init(&mut self, sink: StreamSink<Vec<u8>>) {
        self.inner.init(Box::new(move |data| {
            log::info!("Sending data from rust to flutter {:?}", data);
            if let Err(e) = sink.add(data) {
                todo!("Error sending data to flutter: {:?}", e);
            }
        }))
    }

    pub async fn rx_callback(
        &self,
        data: &[u8],
    ) -> Result<(), ::frb_prpc_juggle::client_interface::WireError> {
        self.inner.rx_callback(data).await
    }
}

impl ClientEndpointInterface for FlutterClient {
    fn call_rpc_endpoint<E: postcard_rpc::Endpoint>(
        &self,
        req: E::Request,
    ) -> impl std::future::Future<
        Output = Result<E::Response, ::frb_prpc_juggle::client_interface::WireError>,
    > + Send
    where
        E::Request: serde::Serialize + postcard_schema::Schema + Send,
        E::Response: serde::de::DeserializeOwned,
    {
        self.inner.call_rpc_endpoint::<E>(req)
    }
}

impl protocol::endpoints::EndpointDispatcher for FlutterClient {}

impl ClientTopicInterface for FlutterClient {
    async fn subscribe<T: Topic>(&self, sink: Box<dyn TopicSink>) -> Result<(), WireError>
    where
        T::Message: DeserializeOwned,
    {
        self.inner.subscribe::<T>(sink).await
    }

    async fn unsubscribe<T: Topic>(&self) -> Result<(), WireError>
    where
        T::Message: DeserializeOwned,
    {
        self.inner.unsubscribe::<T>().await
    }
}

impl TopicDispatcher for FlutterClient {}
