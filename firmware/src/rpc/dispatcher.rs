use core::{cell::RefCell, mem::MaybeUninit};

use defmt_brtt::DefmtConsumer;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Ticker, Timer};
use esp_hal::gpio::{Input, Output};
use maitake_sync::Mutex;
use postcard_rpc::{
    Key, Topic, TopicMap, define_dispatch,
    header::VarHeader,
    server::{Sender, SpawnContext},
};
use protocol::{
    endpoints::*,
    topics::{
        BandwidthTestTopic, BandwidthTestTopicData, ButtonEvent, ButtonEvents, LogMessage,
        SysLogsTopic, SysStatsTopic,
    },
    v1::{MemoryUsage, SysStats},
};

fn get_firmware_version(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::VersionString {
    defmt::debug!("Handling get_firmware_version");
    use core::str::FromStr;
    VersionString::from_str("v0.0.1").expect("Version string too long")
}

async fn get_sys_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::SysSettings {
    defmt::debug!("Handling get_sys_settings");
    let mut ret = protocol::endpoints::SysSettings::new();
    match crate::storage::SYSTEM_CONFIG.get().await {
        Ok(s) => ret.ble_device_name = s.ble_adv_name,
        _ => {}
    }
    ret
}
async fn set_sys_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: protocol::endpoints::SysSettings,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling set_sys_settings");
    let mut new_config = crate::storage::SysConfig::default();
    new_config.ble_adv_name = rqst.ble_device_name;

    if crate::storage::SYSTEM_CONFIG.set(new_config).await.is_err() {
        defmt::error!("Failed to save sys settings");
    }
    protocol::endpoints::EmptyRes {}
}

fn sys_ping(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling sys_ping");
    protocol::endpoints::EmptyRes {}
}

fn sys_stats() -> SysStats {
    let memstats = esp_alloc::HEAP.stats();
    SysStats {
        cpu_usage: crate::stats::cpu_stats(),
        memory_usage: MemoryUsage {
            used: memstats.current_usage as u32,
            total: memstats.size as u32,
        },
        uptime: Instant::now().as_secs() as u32,
    }
}

#[embassy_executor::task]
async fn start_sys_stats_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_sys_stats_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::SysStatsTopic::TOPIC_KEY);
    topic_stop_signal.try_take(); // clear the pending stop signals (we haven't responded to the start request yet.)
    let _guard = crate::stats::start_collecting_stats();
    if let Err(err) = sender
        .reply::<StartSysStatsTopic>(header.seq_no, &(().into()))
        .await
    {
        defmt::error!(
            "Failed to reply to start_sys_stats_topic rpc message {}",
            err
        );
        return;
    }

    let mut seq = 0u8;
    loop {
        let stats = sys_stats();

        if let Err(err) = sender.publish::<SysStatsTopic>(seq.into(), &stats).await {
            defmt::error!("Send error! {}", err);
            break;
        }
        seq = seq.wrapping_add(1);
        match select(crate::stats::wait_new_cpu_stats(), topic_stop_signal.wait()).await {
            Either::First(_) => continue,
            Either::Second(_) => break,
        }
    }
}

fn stop_sys_stats_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling stop_sys_stats_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::SysStatsTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    protocol::endpoints::EmptyRes {}
}

#[embassy_executor::task]
async fn start_sys_logs_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_sys_logs_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::SysLogsTopic::TOPIC_KEY);
    topic_stop_signal.try_take(); // clear the pending stop signals (we haven't responded to the start request yet.)
    if let Err(err) = sender
        .reply::<StartSysLogsTopic>(header.seq_no, &(().into()))
        .await
    {
        defmt::error!(
            "Failed to reply to start_sys_stats_topic rpc message {}",
            err
        );
        return;
    }

    let mut seq = 0u8;

    // Only one task exists and this is the only place we borrow. This is exclusive:
    let mut logger = context.logger.borrow_mut();

    loop {
        // let logs_to_send: LogMessage = Default::default();

        let logs_to_send = match select(logger.wait_for_log(), topic_stop_signal.wait()).await {
            Either::First(grant) => {
                // Here we are sending the log messages. Don't log anything in this block
                let mut log_msg = LogMessage::default();
                log_msg.defmt_bytes.clear();
                log_msg
                    .defmt_bytes
                    .extend_from_slice(grant.buf())
                    .expect("defmt-brtt buffer should fit in LogMessage");
                grant.release(log_msg.defmt_bytes.len());
                log_msg
            }
            Either::Second(_) => break,
        };
        if let Err(err) = sender
            .publish::<SysLogsTopic>(seq.into(), &logs_to_send)
            .await
        {
            defmt::error!("Send error! {}", err);
            break;
        }
        seq = seq.wrapping_add(1);
    }
}

fn stop_sys_logs_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling stop_sys_logs_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::SysLogsTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    protocol::endpoints::EmptyRes {}
}

async fn get_mtu(context: &mut DispatchContext, _header: VarHeader, _rqst: NoArg) -> u16 {
    defmt::debug!("Handling get_mtu");
    context.tx.get_current_mtu().await.unwrap_or(0)
}

// Application Endpoints
async fn get_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::ApplSettings {
    defmt::debug!("Handling get_appl_settings");
    let mut a_settings = protocol::endpoints::ApplSettings::new();
    match crate::storage::APP_CONFIG.get().await {
        Ok(a) => a_settings.led_blink_duration_ms = a.led_blink_duration,
        _ => {}
    }
    a_settings
}

async fn set_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: protocol::endpoints::ApplSettings,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling set_appl_settings");
    let mut new_config = crate::storage::ApplicationConfig::default();
    new_config.led_blink_duration = rqst.led_blink_duration_ms;

    if crate::storage::APP_CONFIG.set(new_config).await.is_err() {
        defmt::error!("Failed to save app settings");
    }
    protocol::endpoints::EmptyRes {}
}

async fn blink_led_n_times(
    context: &mut DispatchContext,
    _header: VarHeader,
    rqst: u8,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling blink_led_n_times");
    let times = rqst;
    let led = &mut context.led;
    let delay = {
        crate::storage::APP_CONFIG
            .get_or_default()
            .await
            .led_blink_duration
    };
    let mut ticker = Ticker::every(Duration::from_millis(delay as u64));
    for _ in 0..times {
        led.set_high();
        ticker.next().await;
        led.set_low();
        ticker.next().await;
    }
    protocol::endpoints::EmptyRes {}
}

const DEBOUNCE_TIME_MS: u64 = 20;
async fn button_events_topic_worker(button: &Mutex<Input<'static>>, sender: Sender<BleWireTxImpl>) {
    let mut seq = 0u8;
    loop {
        let pressed_for = {
            let mut button = button.lock().await;
            button.wait_for_low().await;
            let pressed_at = Instant::now();
            Timer::after_millis(DEBOUNCE_TIME_MS).await;
            button.wait_for_high().await;
            Instant::now() - pressed_at
        }
        .as_millis();
        if sender
            .publish::<ButtonEvents>(
                seq.into(),
                &ButtonEvent {
                    press_time_in_ms: pressed_for as u32,
                },
            )
            .await
            .is_err()
        {
            defmt::error!("Send error!"); // assume connection closed
            break;
        }
        seq = seq.wrapping_add(1);
        Timer::after_millis(DEBOUNCE_TIME_MS).await;
    }
}

#[embassy_executor::task]
async fn start_button_events_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_button_events_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::ButtonEvents::TOPIC_KEY);
    // Consume any pending stop signal, these have arrived before we started (we started the task, we are in it,
    // but we haven't responded to the spawn Rpc request yet. So for the client we are still starting...)
    let _ = topic_stop_signal.try_take();
    if sender
        .reply::<StartButtonEventsTopic>(header.seq_no, &(().into()))
        .await
        .is_err()
    {
        defmt::error!("Failed to reply to start_button_events_topic rpc message");
        return;
    }
    match select(
        button_events_topic_worker(&context.button, sender),
        topic_stop_signal.wait(),
    )
    .await
    {
        Either::First(_) => {
            defmt::debug!("Button events topic loop stopped prematurely");
        }
        Either::Second(_) => {
            defmt::debug!("Stop signal for button events topic");
        }
    };
}

async fn stop_button_events_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::EmptyRes {
    defmt::error!("Handling stop_button_events_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::ButtonEvents::TOPIC_KEY);
    topic_stop_signal.signal(());
    protocol::endpoints::EmptyRes {}
}

fn echo(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: EchoRequest,
) -> protocol::endpoints::EchoResponse {
    defmt::debug!("Handling echo");
    protocol::endpoints::EchoResponse { inner: rqst.inner }
}

const fn topic_state<const SIZE: usize>(map: TopicMap) -> [(&'static Key, TopicStopSignal); SIZE] {
    let mut table: [MaybeUninit<(&'static Key, TopicStopSignal)>; SIZE] =
        MaybeUninit::uninit().transpose();
    let mut i = 0;
    while i < SIZE {
        table[i].write((&map.topics[i].1, TopicStopSignal::new()));
        i += 1;
    }

    assert!(
        size_of_val(&table) == size_of::<[(&'static Key, TopicStopSignal); SIZE]>(),
        "The use of transmute_copy is not correct."
    );
    unsafe { core::mem::transmute_copy::<_, [(&'static Key, TopicStopSignal); SIZE]>(&table) }
}

pub type TopicStopSignal = Signal<CriticalSectionRawMutex, ()>;
#[derive(Copy, Clone)]
pub struct TopicTaskTable(&'static [(&'static Key, TopicStopSignal)]);
impl TopicTaskTable {
    fn stop_signal(&self, topic_key: postcard_rpc::Key) -> &TopicStopSignal {
        self.0
            .iter()
            .find(|(key, _)| **key == topic_key)
            .map(|(_key, signal)| signal)
            .expect("Every topic has to have a topic task entry")
    }
}
static TOPIC_TASK_STATE: [(&'static Key, TopicStopSignal); protocol::topics::TOPICS.topics.len()] =
    topic_state(protocol::topics::TOPICS);

pub(crate) struct DispatchContext {
    task_table: TopicTaskTable,
    pub(crate) button: &'static Mutex<Input<'static>>,
    pub(crate) led: Output<'static>,
    pub(crate) logger: &'static RefCell<DefmtConsumer>,
    tx: BleWireTxImpl,
}

impl DispatchContext {
    pub fn new(
        button: &'static Mutex<Input<'static>>,
        led: Output<'static>,
        logger: &'static RefCell<DefmtConsumer>,
        tx: BleWireTxImpl,
    ) -> Self {
        Self {
            task_table: TopicTaskTable(&TOPIC_TASK_STATE),
            button,
            led,
            logger,
            tx,
        }
    }
}

pub struct DispatchSpawnContext {
    pub task_table: TopicTaskTable,
    pub button: &'static Mutex<Input<'static>>,
    pub logger: &'static RefCell<DefmtConsumer>,
}

impl SpawnContext for DispatchContext {
    type SpawnCtxt = DispatchSpawnContext;

    fn spawn_ctxt(&mut self) -> Self::SpawnCtxt {
        DispatchSpawnContext {
            task_table: self.task_table,
            button: self.button,
            logger: self.logger,
        }
    }
}

// importing these handlers to the namespace because the `define_dispatch` macro does not match
// fully qualified paths for handler functions, it expects identifiers
use crate::{
    ble::BleWireTxImpl,
    rpc::ota::{
        approve_firmware as ota_approve_firmware, factory_reset as ota_factory_reset,
        finalize as ota_finalize, prepare as ota_prepare, transfer_bytes as ota_transfer_bytes,
    },
};
use postcard_rpc::server::impls::embedded_io_async_v0_6::dispatch_impl::{WireSpawnImpl, spawn_fn};

#[embassy_executor::task]
async fn start_bandwidth_test_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_bandwidth_test_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::BandwidthTestTopic::TOPIC_KEY);

    let _ = topic_stop_signal.try_take();
    if sender
        .reply::<StartTestTopicBandwidth>(header.seq_no, &(().into()))
        .await
        .is_err()
    {
        defmt::error!("Failed to reply to start_bandwidth_test_topic rpc message");
        return;
    }

    let mut seq = 0u8;
    loop {
        let data = BandwidthTestTopicData::new(Instant::now().as_ticks() as u8);
        if sender
            .publish::<BandwidthTestTopic>(seq.into(), &data)
            .await
            .is_err()
        {
            break;
        }
        seq = seq.wrapping_add(1);

        match select(Timer::after_ticks(0), topic_stop_signal.wait()).await {
            Either::First(_) => {
                continue;
            }
            Either::Second(_) => {
                defmt::debug!("Stop signal for bandwidth test topic");
                break;
            }
        };
    }
}

async fn stop_bandwidth_test_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::EmptyRes {
    defmt::error!("Handling stop_bandwidth_test_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(protocol::topics::BandwidthTestTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    protocol::endpoints::EmptyRes {}
}

fn do_nothing(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: BandwidthTestData,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling do_nothing");
    protocol::endpoints::EmptyRes {}
}

define_dispatch! {
    app: BleDispatcher;
    spawn_fn: spawn_fn;
    tx_impl: BleWireTxImpl;
    spawn_impl: WireSpawnImpl;
    context: DispatchContext;

    endpoints: {
        list: protocol::endpoints::ENDPOINT_LIST;

        | EndpointTy                | kind      | handler                       |
        | ----------                | ----      | -------                       |
        | GetFirmwareVersion        | blocking  | get_firmware_version          |
        | GetSysSettings      | async               | get_sys_settings      |
        | SetSysSettings      | async               | set_sys_settings      |
        | PingEndpoint        | blocking            | sys_ping              |
        | StartSysStatsTopic  | spawn               | start_sys_stats_topic |
        | StopSysStatsTopic   | blocking            | stop_sys_stats_topic  |
        | StartSysLogsTopic   | spawn               | start_sys_logs_topic  |
        | StopSysLogsTopic    | blocking            | stop_sys_logs_topic   |
        | GetMtu              | async               | get_mtu               |
        | PrepareOta          | async | ota_prepare          |
        | TransferOtaBytes    | async | ota_transfer_bytes   |
        | FinalizeOta         | async | ota_finalize         |
        | ApproveFirmware     | async | ota_approve_firmware |
        | FactoryReset        | async | ota_factory_reset    |
        // Application Endpoints
        | GetApplSettings         | async             | get_appl_settings  |
        | SetApplSettings         | async             | set_appl_settings  |
        | BlinkLedEndpoint        | async             | blink_led_n_times  |
        | StartButtonEventsTopic  | spawn             | start_button_events_topic |
        | StopButtonEventsTopic   | async             | stop_button_events_topic  |
        | EchoEndpoint            | blocking          | echo                      |
        | StartTestTopicBandwidth | spawn             | start_bandwidth_test_topic           |
        | StopTestTopicBandwidth  | async             | stop_bandwidth_test_topic            |
        | TestBandwidth           | blocking          | do_nothing                           |
    };
    topics_in: {
        list: protocol::topics::EMPTY_TOPICS;

        | TopicTy                   | kind      | handler                       |
        | ----------                | ----      | -------                       |
    };
    topics_out: {
        list: protocol::topics::TOPICS;
    };
}
