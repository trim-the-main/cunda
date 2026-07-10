use std::sync::{atomic::AtomicBool, Arc};

use defmt_parser::Level;
use frb_prpc_juggle::client_interface::{
    ClientEndpointInterface, ClientTopicInterface, FrbPostcardRpcError, TopicSink,
};
use postcard_rpc::Topic;
use protocol::{
    cunda_common::v1::{endpoints::*, topics::*},
    devices::demo_esp32::v1::{endpoints::*, topics::*, types::*},
};
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

use super::super::demo_esp32::DemoAppTopics;
use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecodingError},
    devices::mocks::{base::MockClient, helpers::PeriodicTopicOutput},
    frb_generated::StreamSink,
    rpc::CundaSysT,
};

#[flutter_rust_bridge::frb(opaque)]
pub struct MockDemoV1Client {
    mc: MockClient,
    current_application_settings: Mutex<ApplSettings>,
}

impl MockDemoV1Client {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        log::info!("Creating a new fake client");
        let log_table: Vec<(Level, &'static str)> = vec![];
        Self {
            mc: MockClient::new("demo_esp32", 1, 1, log_table),
            current_application_settings: Mutex::new(ApplSettings {
                led_blink_duration_ms: 400,
            }),
        }
    }
}

impl DemoAppEndpoints for MockDemoV1Client {
    async fn get_appl_settings(&self, _req: NoArg) -> Result<ApplSettings, FrbPostcardRpcError> {
        log::debug!("Get appl settings called");
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        Ok(self.current_application_settings.lock().await.clone())
    }

    async fn set_appl_settings(&self, req: ApplSettings) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Set appl settings called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        {
            let mut guard = self.current_application_settings.lock().await;
            *guard = req;
        }
        Ok(EmptyRes {})
    }

    async fn blink_led_endpoint(&self, req: u8) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Blink led endpoint called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        let blink_time_ms = self
            .current_application_settings
            .lock()
            .await
            .led_blink_duration_ms as u64;
        for _ in 0..req {
            log::info!("Turning on LED");
            tokio::time::sleep(std::time::Duration::from_millis(blink_time_ms)).await;
            log::info!("Turning off LED");
        }
        Ok(EmptyRes {})
    }

    async fn start_button_events_topic(
        &self,
        _req: NoArg,
    ) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::info!("Start button events topic called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(running) = self
            .mc
            .streams_running_status
            .lock()
            .await
            .get(&ButtonEvents::TOPIC_KEY)
        {
            running.store(true, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn stop_button_events_topic(&self, _req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Stop button events topic called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(flag) = self
            .mc
            .streams_running_status
            .lock()
            .await
            .get(&ButtonEvents::TOPIC_KEY)
        {
            flag.store(false, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn start_test_topic_bandwidth(
        &self,
        _req: NoArg,
    ) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Start test topic bandwidth called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(running) = self
            .mc
            .streams_running_status
            .lock()
            .await
            .get(&BandwidthTestTopic::TOPIC_KEY)
        {
            running.store(true, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn stop_test_topic_bandwidth(
        &self,
        _req: NoArg,
    ) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Stop test topic bandwidth called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(flag) = self
            .mc
            .streams_running_status
            .lock()
            .await
            .get(&BandwidthTestTopic::TOPIC_KEY)
        {
            flag.store(false, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn echo_endpoint(&self, req: EchoRequest) -> Result<EchoResponse, FrbPostcardRpcError> {
        log::debug!("Echo endpoint called");
        Ok(EchoResponse { inner: req.inner })
    }

    async fn test_bandwidth(
        &self,
        _req: BandwidthTestData,
    ) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Test bandwidth called");
        Ok(EmptyRes {})
    }
}

impl DemoAppTopics for MockDemoV1Client {
    async fn create_button_events_stream(
        &self,
        sink: StreamSink<ButtonEvent>,
    ) -> Result<(), FrbPostcardRpcError> {
        let button_stream = PeriodicTopicOutput::<ButtonEvent, _>::new(
            [
                ButtonEvent {
                    press_time_in_ms: 100,
                },
                ButtonEvent {
                    press_time_in_ms: 200,
                },
                ButtonEvent {
                    press_time_in_ms: 300,
                },
                ButtonEvent {
                    press_time_in_ms: 400,
                },
                ButtonEvent {
                    press_time_in_ms: 500,
                },
            ],
            core::time::Duration::from_secs(3),
        );

        let mut streams_running_status_guard = self.mc.streams_running_status.lock().await;
        let keep_notifying = streams_running_status_guard
            .entry(ButtonEvents::TOPIC_KEY)
            .or_insert(Arc::new(AtomicBool::new(true))); //always on
        let handle = button_stream
            .leak_into_sink(sink, keep_notifying.clone())
            .await;
        self.mc.topic_join_handles.lock().await.push(handle);

        Ok(())
    }

    async fn create_bandwidth_test_topic_stream(
        &self,
        sink: StreamSink<BandwidthTestTopicData>,
    ) -> Result<(), FrbPostcardRpcError> {
        let bw_stream = PeriodicTopicOutput::<BandwidthTestTopicData, _>::new(
            [BandwidthTestTopicData::new(0)],
            core::time::Duration::from_millis(100),
        );

        let mut streams_running_status_guard = self.mc.streams_running_status.lock().await;
        let keep_notifying = streams_running_status_guard
            .entry(BandwidthTestTopic::TOPIC_KEY)
            .or_insert(Arc::new(AtomicBool::new(false)));
        let handle = bw_stream.leak_into_sink(sink, keep_notifying.clone()).await;
        self.mc.topic_join_handles.lock().await.push(handle);

        Ok(())
    }
}

// The rest is passthrough traits - boilerplate...
impl protocol::cunda_common::CundaDevice for MockDemoV1Client {
    async fn get_device_id(&self, req: NoArg) -> Result<DeviceId, FrbPostcardRpcError> {
        self.mc.get_device_id(req).await
    }
}

impl CundaSysE for MockDemoV1Client {
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

impl CundaSysT for MockDemoV1Client {
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

impl crate::log_decoder::LogDecoder for MockDemoV1Client {
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

impl ClientEndpointInterface for MockDemoV1Client {
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

impl ClientTopicInterface for MockDemoV1Client {
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
