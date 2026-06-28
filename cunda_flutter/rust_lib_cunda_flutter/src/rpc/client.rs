use frb_prpc_juggle::client_interface::Client;

use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecoderDefmt, LogDecodingError},
    frb_generated::StreamSink,
    rpc::{CundaSysT, LogDecoder},
};

pub struct FlutterClient {
    inner: Client,
    log_decoder: Option<LogDecoderDefmt>,
}

impl AsRef<Client> for FlutterClient {
    fn as_ref(&self) -> &Client {
        &self.inner
    }
}

impl protocol::cunda_common::CundaDevice for FlutterClient {}

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

impl<C> LogDecoder for C
where
    C: AsMut<FlutterClient> + AsRef<FlutterClient>,
{
    fn init_log_decoder(&mut self, table_bytes: &[u8], loc_bytes: &[u8]) {
        self.as_mut().init_log_decoder(table_bytes, loc_bytes)
    }

    fn decode_log(&self, bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError> {
        self.as_ref().decode_log(bytes)
    }
}

pub struct DemoClientV1(FlutterClient);
impl DemoClientV1 {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new(fc: FlutterClient) -> Self {
        Self(fc)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cunda_device_type() -> String {
        String::from(protocol::devices::demo_esp32::DEVICE_TYPE)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cunda_common_protocol() -> u32 {
        protocol::devices::demo_esp32::v1::RPC_PROTOCOL_VERSION
    }
}

impl AsRef<Client> for DemoClientV1 {
    fn as_ref(&self) -> &Client {
        &self.0.as_ref()
    }
}

impl AsMut<FlutterClient> for DemoClientV1 {
    fn as_mut(&mut self) -> &mut FlutterClient {
        &mut self.0
    }
}

impl AsRef<FlutterClient> for DemoClientV1 {
    fn as_ref(&self) -> &FlutterClient {
        &self.0
    }
}

impl protocol::cunda_common::v1::endpoints::CundaSysE for DemoClientV1 {}
impl protocol::devices::demo_esp32::v1::endpoints::DemoAppEndpoints for DemoClientV1 {}

impl CundaSysT for DemoClientV1 {}
protocol::devices::demo_esp32::v1::topics::define_topic_trait!(DemoAppTopics with StreamSink);
impl DemoAppTopics for DemoClientV1 {}
