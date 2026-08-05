use defmt_parser::Level;
use frb_prpc_juggle::client_interface::{
    ClientEndpointInterface, ClientTopicInterface, FrbPostcardRpcError, TopicSink,
};
use postcard_rpc::Topic;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecodingError},
    devices::mocks::base::MockClient,
    frb_generated::StreamSink,
    rpc::{CundaGpsT, CundaSysT},
};

use super::super::nokta::NoktaTopics;
use protocol::{
    cunda_common::v1::{endpoints::*, topics::sys::*},
    devices::nokta::v1::{endpoints::*, types::*},
};

#[flutter_rust_bridge::frb(opaque)]
pub struct MockNoktaV1Client {
    mc: MockClient,
    current_application_settings: Mutex<NoktaSettings>,
}

impl MockNoktaV1Client {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        log::info!("Creating a new fake client");
        let log_table: Vec<(Level, &'static str)> = vec![];
        Self {
            mc: MockClient::new("nokta", 1, 1, log_table),
            current_application_settings: Mutex::new(NoktaSettings {
                led_blink_duration_ms: 400,
            }),
        }
    }
}

impl NoktaEndpoints for MockNoktaV1Client {
    async fn get_appl_settings(&self, _req: NoArg) -> Result<NoktaSettings, FrbPostcardRpcError> {
        log::debug!("Get appl settings called");
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        Ok(self.current_application_settings.lock().await.clone())
    }

    async fn set_appl_settings(&self, req: NoktaSettings) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Set appl settings called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        {
            let mut guard = self.current_application_settings.lock().await;
            *guard = req;
        }
        Ok(EmptyRes {})
    }
}

impl NoktaTopics for MockNoktaV1Client {}

// The rest is passthrough traits - boilerplate...
impl protocol::cunda_common::CundaDevice for MockNoktaV1Client {
    async fn get_device_id(&self, req: NoArg) -> Result<DeviceId, FrbPostcardRpcError> {
        self.mc.get_device_id(req).await
    }
}

impl CundaSysE for MockNoktaV1Client {
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

impl CundaGpsE for MockNoktaV1Client {
    async fn start_raw_nmea_topic(
        &self,
        req: NoArg,
    ) -> ::core::result::Result<EmptyRes, ::frb_prpc_juggle::client_interface::FrbPostcardRpcError>
    {
        self.mc.start_raw_nmea_topic(req).await
    }

    async fn stop_raw_nmea_topic(
        &self,
        req: NoArg,
    ) -> ::core::result::Result<EmptyRes, ::frb_prpc_juggle::client_interface::FrbPostcardRpcError>
    {
        self.mc.stop_raw_nmea_topic(req).await
    }

    async fn start_parsed_gps_topic(
        &self,
        req: NoArg,
    ) -> ::core::result::Result<EmptyRes, ::frb_prpc_juggle::client_interface::FrbPostcardRpcError>
    {
        self.mc.start_parsed_gps_topic(req).await
    }

    async fn stop_parsed_gps_topic(
        &self,
        req: NoArg,
    ) -> ::core::result::Result<EmptyRes, ::frb_prpc_juggle::client_interface::FrbPostcardRpcError>
    {
        self.mc.stop_parsed_gps_topic(req).await
    }
}

impl CundaGpsT for MockNoktaV1Client {
    async fn create_raw_nmea_topic_stream(
        &self,
        sink: StreamSink<RawNmea0183Sentence>,
    ) -> Result<(), FrbPostcardRpcError> {
        self.mc.create_raw_nmea_topic_stream(sink).await
    }

    async fn create_parsed_gps_topic_stream(
        &self,
        sink: StreamSink<GpsDataWire>,
    ) -> Result<(), FrbPostcardRpcError> {
        self.mc.create_parsed_gps_topic_stream(sink).await
    }
}

impl CundaSysT for MockNoktaV1Client {
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

impl crate::log_decoder::LogDecoder for MockNoktaV1Client {
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

impl ClientEndpointInterface for MockNoktaV1Client {
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

impl ClientTopicInterface for MockNoktaV1Client {
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
