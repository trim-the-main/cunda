/// For manual testing without BLE connection
///
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
};

use frb_prpc_juggle::client_interface::{
    ClientEndpointInterface, ClientTopicInterface, FrbPostcardRpcError, TopicSink,
};
use postcard_rpc::{Key, Topic};
use protocol::{endpoints::*, topics::*, types::DeviceId, v1::*};
use rand::Rng;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

use crate::{
    frb_generated::{self, StreamSink},
    rpc::TopicDispatcher,
};

#[flutter_rust_bridge::frb(opaque)]
pub struct DummyFlutterProtocolClient {
    current_system_settings: Mutex<SysSettings>,
    current_application_settings: Mutex<ApplSettings>,
    streams_running_status: Mutex<HashMap<Key, Arc<AtomicBool>>>,
    topic_join_handles: Mutex<Vec<flutter_rust_bridge::JoinHandle<()>>>,
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

impl EndpointDispatcher for DummyFlutterProtocolClient {
    async fn get_device_id(&self, _req: NoArg) -> Result<DeviceId, FrbPostcardRpcError> {
        log::debug!("Get device id called");
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        Ok(DeviceId::default())
    }

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

impl TopicDispatcher for DummyFlutterProtocolClient {
    async fn create_sys_stats_topic_stream(
        &self,
        sink: StreamSink<SysStats>,
    ) -> Result<(), FrbPostcardRpcError> {
        let mut streams_running_status_guard = self.streams_running_status.lock().await;
        if streams_running_status_guard.contains_key(&SysStatsTopic::TOPIC_KEY) {
            return Err(FrbPostcardRpcError::AlreadySubscribedtoTopic);
        }
        let sys_stream = PeriodicTopicOutput::<SysStats, _>::new(
            [
                SysStats {
                    uptime: 1,
                    cpu_usage: protocol::v1::CpuUsage {
                        core0: Percent(10),
                        core1: Percent(50),
                    },
                    memory_usage: MemoryUsage {
                        used: 12_542,
                        total: 96_123,
                    },
                },
                SysStats {
                    uptime: 2,
                    cpu_usage: protocol::v1::CpuUsage {
                        core0: Percent(10),
                        core1: Percent(11),
                    },
                    memory_usage: MemoryUsage {
                        used: 24532,
                        total: 96_123,
                    },
                },
            ],
            core::time::Duration::from_secs(1),
        );
        let keep_notifying = {
            streams_running_status_guard
                .insert(SysStatsTopic::TOPIC_KEY, Arc::new(AtomicBool::new(false)));
            streams_running_status_guard
                .get(&SysStatsTopic::TOPIC_KEY)
                .unwrap()
        };
        let handle: flutter_rust_bridge::JoinHandle<()> = sys_stream
            .leak_into_sink(sink, keep_notifying.clone())
            .await;

        let mut join_handles = self.topic_join_handles.lock().await;
        join_handles.push(handle);

        Ok(())
    }

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
        let keep_notifying = {
            streams_running_status_guard
                .insert(ButtonEvents::TOPIC_KEY, Arc::new(AtomicBool::new(true))); //always on
            streams_running_status_guard
                .get(&ButtonEvents::TOPIC_KEY)
                .unwrap()
        };

        let handle = button_stream
            .leak_into_sink(sink, keep_notifying.clone())
            .await;
        self.topic_join_handles.lock().await.push(handle);

        Ok(())
    }
}
