/// For manual testing without BLE connection
///
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
};

use frb_prpc_juggle::client_interface::{
    ClientEndpointInterface, ClientTopicInterface, FrbPostcardRpcError, TopicSink,
};
use postcard_rpc::{Key, Topic};
use protocol::{
    cunda_common::v1::{endpoints::*, topics::*, types},
    devices::demo_esp32::v1::{endpoints::*, topics::*, types::*},
};
use rand::Rng;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecodingError},
    frb_generated::{self, StreamSink},
    rpc::{client::DemoAppTopics, CundaSysT},
};

#[flutter_rust_bridge::frb(opaque)]
pub struct DummyFlutterProtocolClient {
    current_system_settings: Mutex<SysSettings>,
    current_application_settings: Mutex<ApplSettings>,
    streams_running_status: Mutex<HashMap<Key, Arc<AtomicBool>>>,
    topic_join_handles: Mutex<Vec<flutter_rust_bridge::JoinHandle<()>>>,
    log_decoder_initialized: AtomicBool,
    log_counter: AtomicU32,
}

impl DummyFlutterProtocolClient {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        log::info!("Creating a new dummy client");
        Self {
            current_system_settings: Mutex::new(SysSettings {
                ble_device_name: String::from_str("dummy").expect("The string is valid"),
                ..Default::default()
            }),
            current_application_settings: Mutex::new(ApplSettings {
                led_blink_duration_ms: 400,
            }),
            streams_running_status: Mutex::new(HashMap::new()),
            topic_join_handles: Mutex::new(Vec::new()),
            log_decoder_initialized: AtomicBool::new(false),
            log_counter: AtomicU32::new(0),
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn init(&mut self, _sink: StreamSink<Vec<u8>>) {}

    pub async fn rx_callback(&self, data: &[u8]) -> Result<(), FrbPostcardRpcError> {
        log::trace!("RUST RECEIVED DATA: {:?}", data);
        Ok(())
    }
}

impl Drop for DummyFlutterProtocolClient {
    fn drop(&mut self) {
        log::info!("Dropping the dummy client");
        for (_, v) in self.streams_running_status.get_mut().iter_mut() {
            v.store(false, std::sync::atomic::Ordering::Release);
        }
        for handle in self.topic_join_handles.get_mut().iter_mut() {
            handle.abort();
        }
    }
}

impl ClientEndpointInterface for DummyFlutterProtocolClient {
    async fn call_rpc_endpoint<E: postcard_rpc::Endpoint>(
        &self,
        _req: E::Request,
    ) -> Result<E::Response, FrbPostcardRpcError>
    where
        E::Request: serde::Serialize + postcard_schema::Schema + Send,
        E::Response: serde::de::DeserializeOwned,
    {
        Err(FrbPostcardRpcError::InternalError)
    }
}

impl protocol::cunda_common::CundaDevice for DummyFlutterProtocolClient {
    async fn get_device_id(&self, _req: NoArg) -> Result<DeviceId, FrbPostcardRpcError> {
        log::debug!("Get device id called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        Ok(DeviceId::new(
            "cunda",
            1,
            1,
            "0.0.0",
            1,
            Some(
                types::GitRevSha::from_hex_str("deadbeef01234567feedbacc89012345deadfaad").unwrap(),
            ),
        ))
    }
}

impl CundaSysE for DummyFlutterProtocolClient {
    async fn get_sys_settings(&self, _req: NoArg) -> Result<SysSettings, FrbPostcardRpcError> {
        log::debug!("Get sys settings called");
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        Ok(self.current_system_settings.lock().await.clone())
    }

    async fn set_sys_settings(&self, req: SysSettings) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Set sys settings called setting to: {:?}", req);
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        {
            let mut guard = self.current_system_settings.lock().await;
            *guard = req;
        }
        //
        Ok(EmptyRes {})
    }

    async fn ping_endpoint(&self, _req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        let random_millisecs = {
            let mut rng = rand::rng();
            rng.random_range(100..500)
        };
        log::debug!("Ping delaying for {random_millisecs}ms");
        tokio::time::sleep(std::time::Duration::from_millis(random_millisecs)).await;
        Ok(EmptyRes {})
    }

    async fn start_sys_stats_topic(&self, _req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Start sys stats topic called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(running) = self
            .streams_running_status
            .lock()
            .await
            .get(&SysStatsTopic::TOPIC_KEY)
        {
            running.store(true, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn stop_sys_stats_topic(&self, _req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Stop sys stats topic called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(flag) = self
            .streams_running_status
            .lock()
            .await
            .get(&SysStatsTopic::TOPIC_KEY)
        {
            flag.store(false, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn start_sys_logs_topic(&self, _req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Start sys logs topic called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(running) = self
            .streams_running_status
            .lock()
            .await
            .get(&SysLogsTopic::TOPIC_KEY)
        {
            running.store(true, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn stop_sys_logs_topic(&self, _req: NoArg) -> Result<EmptyRes, FrbPostcardRpcError> {
        log::debug!("Stop sys logs topic called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if let Some(flag) = self
            .streams_running_status
            .lock()
            .await
            .get(&SysLogsTopic::TOPIC_KEY)
        {
            flag.store(false, std::sync::atomic::Ordering::Release);
            Ok(EmptyRes {})
        } else {
            Err(FrbPostcardRpcError::InternalError)
        }
    }

    async fn get_mtu(&self, _req: NoArg) -> Result<u16, FrbPostcardRpcError> {
        log::debug!("Get MTU called");
        Ok(251)
    }

    async fn prepare_ota(&self, _req: OtaMData) -> Result<OtaResult, FrbPostcardRpcError> {
        log::debug!("Prepare OTA called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        Ok(OtaResult::TransferReady)
    }

    async fn transfer_ota_bytes(&self, _req: OtaBytes) -> Result<OtaResult, FrbPostcardRpcError> {
        Ok(OtaResult::TransferReady)
    }

    async fn finalize_ota(&self, _req: NoArg) -> Result<OtaResult, FrbPostcardRpcError> {
        log::debug!("Finalize OTA called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        Ok(OtaResult::TransferComplete)
    }

    async fn approve_firmware(&self, _req: NoArg) -> Result<OtaResult, FrbPostcardRpcError> {
        log::debug!("Approve firmware called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        Ok(OtaResult::TransferReady)
    }

    async fn factory_reset(&self, _req: NoArg) -> Result<OtaResult, FrbPostcardRpcError> {
        log::debug!("Factory reset called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        Ok(OtaResult::Restarting)
    }
}
impl DemoAppEndpoints for DummyFlutterProtocolClient {
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

impl ClientTopicInterface for DummyFlutterProtocolClient {
    async fn subscribe<T: Topic>(
        &self,
        _sink: Box<dyn TopicSink>,
    ) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        Err(FrbPostcardRpcError::AlreadySubscribedtoTopic)
    }

    async fn unsubscribe<T: Topic>(&self) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        Err(FrbPostcardRpcError::NotSubscribedToTopic)
    }
}

struct PeriodicTopicOutput<T, const N: usize> {
    pub list_of_outputs: [T; N],
    pub every: core::time::Duration,
}
impl<T, const N: usize> PeriodicTopicOutput<T, N>
where
    T: frb_generated::SseEncode + Clone + Send + std::fmt::Debug + 'static,
{
    fn new(list_of_outputs: [T; N], every: core::time::Duration) -> Self {
        Self {
            list_of_outputs,
            every,
        }
    }
    async fn leak_into_sink(
        self,
        sink: StreamSink<T>,
        keep_notifying: Arc<AtomicBool>,
    ) -> flutter_rust_bridge::JoinHandle<()> {
        log::info!(
            "Starting periodic topic output, first output will be {:?}",
            self.list_of_outputs[0]
        );
        flutter_rust_bridge::spawn(async move {
            let mut interval = tokio::time::interval(self.every);
            let mut i = 0;
            loop {
                interval.tick().await;
                i = (i + 1) % N;
                if keep_notifying.load(core::sync::atomic::Ordering::Acquire) {
                    sink.add(self.list_of_outputs[i].clone()).unwrap();
                }
            }
        })
    }
}

impl CundaSysT for DummyFlutterProtocolClient {
    async fn create_sys_stats_topic_stream(
        &self,
        sink: StreamSink<SysStats>,
    ) -> Result<(), FrbPostcardRpcError> {
        let sys_stream = PeriodicTopicOutput::<SysStats, _>::new(
            [
                SysStats {
                    uptime: 1,
                    cpu_usage: types::CpuUsage {
                        core0: types::Percent(10),
                        core1: types::Percent(50),
                    },
                    memory_usage: types::MemoryUsage {
                        used: 12_542,
                        total: 96_123,
                    },
                },
                SysStats {
                    uptime: 2,
                    cpu_usage: types::CpuUsage {
                        core0: types::Percent(10),
                        core1: types::Percent(11),
                    },
                    memory_usage: types::MemoryUsage {
                        used: 24532,
                        total: 96_123,
                    },
                },
            ],
            core::time::Duration::from_secs(1),
        );

        let mut streams_running_status_guard = self.streams_running_status.lock().await;
        let keep_notifying = streams_running_status_guard
            .entry(SysStatsTopic::TOPIC_KEY)
            .or_insert(Arc::new(AtomicBool::new(false)));
        let handle: flutter_rust_bridge::JoinHandle<()> = sys_stream
            .leak_into_sink(sink, keep_notifying.clone())
            .await;
        self.topic_join_handles.lock().await.push(handle);

        Ok(())
    }
    async fn create_sys_logs_topic_stream(
        &self,
        sink: StreamSink<LogMessage>,
    ) -> Result<(), FrbPostcardRpcError> {
        let log_stream = PeriodicTopicOutput::<LogMessage, _>::new(
            [LogMessage {
                defmt_bytes: vec![0u8; 8],
            }],
            core::time::Duration::from_secs(2),
        );

        let mut streams_running_status_guard = self.streams_running_status.lock().await;
        let keep_notifying = streams_running_status_guard
            .entry(SysLogsTopic::TOPIC_KEY)
            .or_insert(Arc::new(AtomicBool::new(false)));
        let handle = log_stream
            .leak_into_sink(sink, keep_notifying.clone())
            .await;
        self.topic_join_handles.lock().await.push(handle);

        Ok(())
    }
}

impl DemoAppTopics for DummyFlutterProtocolClient {
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
            core::time::Duration::from_secs(5),
        );

        let mut streams_running_status_guard = self.streams_running_status.lock().await;
        let keep_notifying = streams_running_status_guard
            .entry(ButtonEvents::TOPIC_KEY)
            .or_insert(Arc::new(AtomicBool::new(true))); //always on
        let handle = button_stream
            .leak_into_sink(sink, keep_notifying.clone())
            .await;
        self.topic_join_handles.lock().await.push(handle);

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

        let mut streams_running_status_guard = self.streams_running_status.lock().await;
        let keep_notifying = streams_running_status_guard
            .entry(BandwidthTestTopic::TOPIC_KEY)
            .or_insert(Arc::new(AtomicBool::new(false)));
        let handle = bw_stream.leak_into_sink(sink, keep_notifying.clone()).await;
        self.topic_join_handles.lock().await.push(handle);

        Ok(())
    }
}

impl crate::rpc::LogDecoder for DummyFlutterProtocolClient {
    fn init_log_decoder(&mut self, _table_bytes: &[u8], _loc_bytes: &[u8]) {
        log::info!("Dummy client: log decoder initialized");
        self.log_decoder_initialized
            .store(true, std::sync::atomic::Ordering::Release);
    }

    #[flutter_rust_bridge::frb(sync)]
    fn decode_log(&self, _bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError> {
        if !self
            .log_decoder_initialized
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(LogDecodingError::NoTableData);
        }
        let count = self
            .log_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (level, msg) = match count % 5 {
            0 => (defmt_parser::Level::Info, "System booted successfully"),
            1 => (defmt_parser::Level::Debug, "BLE advertising started"),
            2 => (defmt_parser::Level::Warn, "Memory usage above 80%"),
            3 => (
                defmt_parser::Level::Info,
                "Sensor reading: temperature=23.5C",
            ),
            _ => (
                defmt_parser::Level::Error,
                "Failed to write to flash sector 7",
            ),
        };
        Ok(vec![DefmtLogEntry {
            level: Some(level),
            timestamp: format!("{:.3}", count as f64 * 2.0),
            location: Some("dummy_firmware/src/main.rs:42".to_string()),
            msg: msg.to_string(),
        }])
    }
}
