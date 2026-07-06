/// For manual testing without BLE connection
///
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use frb_prpc_juggle::client_interface::{
    ClientEndpointInterface, ClientTopicInterface, FrbPostcardRpcError, TopicSink,
};
use postcard_rpc::{Key, Topic};
use protocol::cunda_common::v1::{endpoints::*, topics::*, types};
use rand::Rng;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

use super::helpers;
use crate::{
    defmt_log_translation::{DefmtLogEntry, LogDecodingError},
    frb_generated::StreamSink,
    rpc::CundaSysT,
};

#[flutter_rust_bridge::frb(opaque)]
pub struct MockClient {
    device_type: &'static str,
    firmware_version: AtomicU32,
    protocol_version: AtomicU32,
    current_system_settings: Mutex<SysSettings>,
    pub(crate) streams_running_status: Mutex<HashMap<Key, Arc<AtomicBool>>>,
    pub(crate) topic_join_handles: Mutex<Vec<flutter_rust_bridge::JoinHandle<()>>>,
    log_decoder_initialized: bool,
    log_table: Vec<(defmt_parser::Level, &'static str)>,
    log_counter: AtomicU32,
}

impl protocol::cunda_common::CundaDevice for MockClient {
    async fn get_device_id(&self, _req: NoArg) -> Result<types::DeviceId, FrbPostcardRpcError> {
        log::debug!("Get device id called");
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let firmware_version = format!("{}.0.0", self.firmware_version.load(Ordering::Relaxed));
        Ok(types::DeviceId::new(
            self.device_type,
            self.protocol_version
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            firmware_version.as_str(),
            1,
            Some(
                types::GitRevSha::from_hex_str("deadbeef01234567feedbacc89012345deadfaad").unwrap(),
            ),
        ))
    }
}

impl CundaSysE for MockClient {
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
        self.firmware_version.fetch_add(1, Ordering::SeqCst);
        self.protocol_version.fetch_add(1, Ordering::SeqCst);
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

impl CundaSysT for MockClient {
    async fn create_sys_stats_topic_stream(
        &self,
        sink: StreamSink<SysStats>,
    ) -> Result<(), FrbPostcardRpcError> {
        let sys_stream = helpers::PeriodicTopicOutput::<SysStats, _>::new(
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
        let log_stream = helpers::PeriodicTopicOutput::<LogMessage, _>::new(
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

impl crate::log_decoder::LogDecoder for MockClient {
    fn init_log_decoder(
        &mut self,
        _table_bytes: &[u8],
        _loc_bytes: &[u8],
    ) -> Result<(), LogDecodingError> {
        log::info!("Fake client: log decoder initialized");
        if self.log_table.len() == 0 {
            Err(LogDecodingError::TableDeserFailed)
        } else {
            self.log_decoder_initialized = true;
            Ok(())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    fn decode_log(&self, bytes: &[u8]) -> Result<Vec<DefmtLogEntry>, LogDecodingError> {
        if !self.log_decoder_initialized {
            return Err(LogDecodingError::NoTableData);
        }

        if bytes.len() == 0 {
            log::error!("Did not find this log message in fake table");
            return Err(LogDecodingError::MessageDecodeFailed((vec![])));
        }
        let count = self
            .log_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed) as usize;
        let table_size = self.log_table.len();
        if table_size == 0 {
            return Err(LogDecodingError::NoTableData);
        }
        let (level, msg) = self.log_table[count % table_size];
        Ok(vec![DefmtLogEntry {
            level: Some(level),
            timestamp: format!("{:.3}", count as f64 * 2.0),
            location: Some("dummy_firmware/src/main.rs:42".to_string()),
            msg: msg.to_string(),
        }])
    }
}

impl ClientEndpointInterface for MockClient {
    async fn call_rpc_endpoint<E: postcard_rpc::Endpoint>(
        &self,
        _req: E::Request,
    ) -> Result<E::Response, FrbPostcardRpcError>
    where
        E::Request: serde::Serialize + postcard_schema::Schema + Send,
        E::Response: serde::de::DeserializeOwned,
    {
        panic!("Mock client does not implement ClientEndpointInterface, it overrides protocol endpoints one by one");
    }
}

impl ClientTopicInterface for MockClient {
    async fn subscribe<T: Topic>(
        &self,
        _sink: Box<dyn TopicSink>,
    ) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        panic!("Mock client does not implement ClientTopicInterface, it overrides protocol topics one by one");
    }

    async fn unsubscribe<T: Topic>(&self) -> Result<(), FrbPostcardRpcError>
    where
        T::Message: DeserializeOwned,
    {
        panic!("Mock client does not implement ClientTopicInterface, it overrides protocol topics one by one");
    }
}

impl MockClient {
    pub(crate) fn new(
        device_type: &'static str,
        firmware_version: u32,
        protocol_version: u32,
        log_table: Vec<(defmt_parser::Level, &'static str)>,
    ) -> Self {
        log::info!(
            "Creating a new fake client for device_type `{}`",
            device_type
        );
        Self {
            device_type,
            firmware_version: AtomicU32::new(firmware_version),
            protocol_version: AtomicU32::new(protocol_version),
            current_system_settings: Mutex::new(SysSettings {
                ble_device_name: format!("fake-{}", device_type),
                ..Default::default()
            }),
            streams_running_status: Mutex::new(HashMap::new()),
            topic_join_handles: Mutex::new(Vec::new()),
            log_decoder_initialized: false,
            log_table,
            log_counter: AtomicU32::new(0),
        }
    }
}

impl Drop for MockClient {
    fn drop(&mut self) {
        log::info!("Dropping the fake client");
        for (_, v) in self.streams_running_status.get_mut().iter_mut() {
            v.store(false, std::sync::atomic::Ordering::Release);
        }
        for handle in self.topic_join_handles.get_mut().iter_mut() {
            handle.abort();
        }
    }
}
