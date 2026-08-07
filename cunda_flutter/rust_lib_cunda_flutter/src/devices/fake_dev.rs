use defmt_parser::Level;
use frb_prpc_juggle::client_interface::{
    ClientEndpointInterface, ClientTopicInterface, FrbPostcardRpcError, TopicSink,
};
use postcard_rpc::Topic;
use protocol::cunda_common::{
    v1::{endpoints::*, topics::sys::*},
    DeviceId,
};
use serde::de::DeserializeOwned;

use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecodingError},
    devices::mocks::base::MockClient,
    frb_generated::StreamSink,
    rpc::{CundaSysT, FlutterWire},
};

#[flutter_rust_bridge::frb(opaque)]
pub struct FakeDevV1Client {
    mc: MockClient,
}

impl FakeDevV1Client {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        log::info!("Creating a new fake client");
        let log_table: Vec<(Level, &'static str)> = vec![
            (Level::Info, "First log line"),
            (Level::Warn, "Second log line"),
            (Level::Error, "Third log line"),
            (Level::Debug, "Fourth log line"),
            (Level::Trace, "Fifth log line"),
            (Level::Info, "Sixth log line"),
        ];
        Self {
            mc: MockClient::new("fake-dev", 1, 1, log_table),
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cunda_device_type() -> String {
        String::from("fake-dev")
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cunda_rpc_protocol() -> u32 {
        1
    }
}

impl FlutterWire for FakeDevV1Client {
    #[flutter_rust_bridge::frb(sync)]
    fn init(&mut self, _sink: StreamSink<Vec<u8>>) {
        panic!("We do not run init on protocol clients")
    }

    async fn rx_callback(
        &self,
        _data: &[u8],
    ) -> Result<(), frb_prpc_juggle::client_interface::FrbPostcardRpcError> {
        panic!("Fake client must not receive data from BLE wire")
    }
}

impl protocol::cunda_common::CundaDevice for FakeDevV1Client {
    async fn get_device_id(&self, req: NoArg) -> Result<DeviceId, FrbPostcardRpcError> {
        self.mc.get_device_id(req).await
    }
}

impl CundaSysE for FakeDevV1Client {
    async fn get_sys_settings(&self, req: NoArg) -> Result<SysSettings, FrbPostcardRpcError> {
        self.mc.get_sys_settings(req).await
    }

    async fn set_sys_settings(&self, req: SysSettings) -> Result<EmptyRes, FrbPostcardRpcError> {
        self.mc.set_sys_settings(req).await
    }

    async fn ping_endpoint(&self, req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        self.mc.ping_endpoint(req).await
    }

    async fn start_sys_stats_topic(&self, req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        self.mc.start_sys_stats_topic(req).await
    }

    async fn stop_sys_stats_topic(&self, req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        self.mc.stop_sys_stats_topic(req).await
    }

    async fn start_sys_logs_topic(&self, req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        self.mc.start_sys_logs_topic(req).await
    }

    async fn stop_sys_logs_topic(&self, req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        self.mc.stop_sys_logs_topic(req).await
    }

    async fn get_mtu(&self, req: NoArg) -> Result<u16, FrbPostcardRpcError> {
        self.mc.get_mtu(req).await
    }

    async fn prepare_ota(&self, req: OtaMData) -> Result<OtaResult, FrbPostcardRpcError> {
        self.mc.prepare_ota(req).await
    }

    async fn transfer_ota_bytes(&self, req: OtaBytes) -> Result<OtaResult, FrbPostcardRpcError> {
        self.mc.transfer_ota_bytes(req).await
    }

    async fn finalize_ota(&self, req: NoArg) -> Result<OtaResult, FrbPostcardRpcError> {
        self.mc.finalize_ota(req).await
    }

    async fn approve_firmware(&self, req: NoArg) -> Result<OtaResult, FrbPostcardRpcError> {
        self.mc.approve_firmware(req).await
    }

    async fn factory_reset(&self, req: NoArg) -> Result<OtaResult, FrbPostcardRpcError> {
        self.mc.factory_reset(req).await
    }
}

impl CundaSysT for FakeDevV1Client {
    async fn create_sys_stats_topic_stream(
        &self,
        sink: StreamSink<SysStats>,
    ) -> Result<(), FrbPostcardRpcError> {
        self.mc.create_sys_stats_topic_stream(sink).await
    }

    async fn create_sys_logs_topic_stream(
        &self,
        sink: StreamSink<LogMessage>,
    ) -> Result<(), FrbPostcardRpcError> {
        self.mc.create_sys_logs_topic_stream(sink).await
    }
}

impl crate::log_decoder::LogDecoder for FakeDevV1Client {
    fn init_log_decoder(
        &mut self,
        table_bytes: &[u8],
        loc_bytes: &[u8],
    ) -> Result<(), LogDecodingError> {
        self.mc.init_log_decoder(table_bytes, loc_bytes)
    }

    #[flutter_rust_bridge::frb(sync)]
    fn decode_log(&self, bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError> {
        self.mc.decode_log(bytes)
    }
}

impl ClientEndpointInterface for FakeDevV1Client {
    async fn call_rpc_endpoint<E: postcard_rpc::Endpoint>(
        &self,
        req: E::Request,
    ) -> Result<E::Response, FrbPostcardRpcError>
    where
        E::Request: serde::Serialize + postcard_schema::Schema + Send,
        E::Response: serde::de::DeserializeOwned,
    {
        self.mc.call_rpc_endpoint::<E>(req).await
    }
}

impl ClientTopicInterface for FakeDevV1Client {
    async fn subscribe<T: Topic>(&self, sink: Box<dyn TopicSink>) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        self.mc.subscribe::<T>(sink).await
    }

    async fn unsubscribe<T: Topic>(&self) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        self.mc.unsubscribe::<T>().await
    }
}
