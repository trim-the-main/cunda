use core::mem::MaybeUninit;

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
    topics::{ButtonEvent, ButtonEvents, SysStatsTopic},
    v1::{MemoryUsage, SysStats},
};

use crate::rpc::ble_wire::BleWireTx;

async fn get_firmware_version(
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
async fn sys_ping(
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
    sender: Sender<BleWireTx>,
) {
    defmt::debug!("Handling start_sys_stats_topic");
    let Some((_key, topic_stop_signal)) = context
        .task_table
        .iter()
        .find(|(key, _)| **key == protocol::topics::SysStatsTopic::TOPIC_KEY)
    else {
        panic!("We should have one task_table entry for every topic");
    };
    topic_stop_signal.try_take(); // clear the pending stop signals (we haven't responded to the start request yet.)
    let _guard = crate::stats::start_collecting_stats();
    if sender
        .reply::<StartSysStatsTopic>(header.seq_no, &(().into()))
        .await
        .is_err()
    {
        defmt::error!("Failed to reply to start_sys_stats_topic rpc message");
        return;
    }

    let mut seq = 0u8;
    loop {
        let stats = sys_stats();

        if sender
            .publish::<SysStatsTopic>(seq.into(), &stats)
            .await
            .is_err()
        {
            defmt::error!("Send error!");
            break;
        }
        seq = seq.wrapping_add(1);
        match select(crate::stats::wait_new_cpu_stats(), topic_stop_signal.wait()).await {
            Either::First(_) => continue,
            Either::Second(_) => break,
        }
    }
}

async fn stop_sys_stats_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> protocol::endpoints::EmptyRes {
    defmt::debug!("Handling stop_sys_stats_topic");

    let Some((_key, topic_stop_signal)) = context
        .task_table
        .iter()
        .find(|(key, _)| **key == protocol::topics::SysStatsTopic::TOPIC_KEY)
    else {
        panic!("We should have one task_table entry for every topic");
    };
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
async fn button_events_topic_worker(button: &Mutex<Input<'static>>, sender: Sender<BleWireTx>) {
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
    sender: Sender<BleWireTx>,
) {
    defmt::debug!("Handling start_button_events_topic");
    let Some((_key, topic_stop_signal)) = context
        .task_table
        .iter()
        .find(|(key, _)| **key == protocol::topics::ButtonEvents::TOPIC_KEY)
    else {
        panic!("We should have one task_table entry for every topic");
    };
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

    let Some((_key, topic_stop_signal)) = context
        .task_table
        .iter()
        .find(|(key, _)| **key == protocol::topics::ButtonEvents::TOPIC_KEY)
    else {
        panic!("We should have one task_table entry for every topic");
    };
    topic_stop_signal.signal(());
    protocol::endpoints::EmptyRes {}
}
async fn echo(
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
pub type TopicTaskTable = &'static [(&'static Key, TopicStopSignal)];
static TOPIC_TASK_STATE: [(&'static Key, TopicStopSignal); protocol::topics::TOPICS.topics.len()] =
    topic_state(protocol::topics::TOPICS);

pub(crate) struct DispatchContext {
    task_table: TopicTaskTable,
    pub(crate) button: &'static Mutex<Input<'static>>,
    pub(crate) led: Output<'static>,
    tx: BleWireTx,
}

impl DispatchContext {
    pub fn new(
        button: &'static Mutex<Input<'static>>,
        led: Output<'static>,
        tx: BleWireTx,
    ) -> Self {
        Self {
            task_table: &TOPIC_TASK_STATE,
            button,
            led,
            tx,
        }
    }
}

pub struct DispatchSpawnContext {
    pub task_table: TopicTaskTable,
    pub button: &'static Mutex<Input<'static>>,
}
impl SpawnContext for DispatchContext {
    type SpawnCtxt = DispatchSpawnContext;

    fn spawn_ctxt(&mut self) -> Self::SpawnCtxt {
        DispatchSpawnContext {
            task_table: self.task_table,
            button: &self.button,
        }
    }
}

// importing these handlers to the namespace because the `define_dispatch` macro does not match
// fully qualified paths for handler functions, it expects identifiers
use crate::rpc::ota::{
    approve_firmware as ota_approve_firmware, factory_reset as ota_factory_reset,
    finalize as ota_finalize, prepare as ota_prepare, transfer_bytes as ota_transfer_bytes,
};
use postcard_rpc::server::impls::embedded_io_async_v0_6::dispatch_impl::{WireSpawnImpl, spawn_fn};

define_dispatch! {
    app: BleDispatcher;
    spawn_fn: spawn_fn;
    tx_impl: BleWireTx;
    spawn_impl: WireSpawnImpl;
    context: DispatchContext;

    endpoints: {
        list: protocol::endpoints::ENDPOINT_LIST;

        | EndpointTy                | kind      | handler                       |
        | ----------                | ----      | -------                       |
        | GetFirmwareVersion        | async     | get_firmware_version          |
        | GetSysSettings      | async               | get_sys_settings      |
        | SetSysSettings      | async               | set_sys_settings      |
        | PingEndpoint        | async               | sys_ping              |
        | StartSysStatsTopic  | spawn               | start_sys_stats_topic |
        | StopSysStatsTopic   | async               | stop_sys_stats_topic  |
        | GetMtu              | async               | get_mtu               |
        | PrepareOta          | async | ota_prepare          |
        | TransferOtaBytes    | async | ota_transfer_bytes   |
        | FinalizeOta         | async | ota_finalize         |
        | ApproveFirmware     | async | ota_approve_firmware |
        | FactoryReset        | async | ota_factory_reset    |
        // Application Endpoints
        | GetApplSettings        | async             | get_appl_settings  |
        | SetApplSettings        | async             | set_appl_settings  |
        | BlinkLedEndpoint       | async             | blink_led_n_times  |
        | StartButtonEventsTopic | spawn             | start_button_events_topic |
        | StopButtonEventsTopic  | async             | stop_button_events_topic  |
        | EchoEndpoint           | async             | echo                      |
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
