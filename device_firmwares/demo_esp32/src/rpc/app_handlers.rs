use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Instant, Ticker, Timer};
use esp_hal::gpio::Input;
use maitake_sync::Mutex;
use postcard_rpc::{Topic, header::VarHeader, server::Sender};

use protocol::devices::demo_esp32::v1::{endpoints, topics, types};

use crate::{
    ble::BleWireTxImpl,
    rpc::context::{DispatchContext, DispatchSpawnContext},
};

pub(crate) async fn get_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::ApplSettings {
    defmt::debug!("Handling get_appl_settings");
    let mut a_settings = types::ApplSettings::new();
    match crate::storage::APP_CONFIG.get().await {
        Ok(a) => a_settings.led_blink_duration_ms = a.led_blink_duration,
        _ => {}
    }
    a_settings
}

pub(crate) async fn set_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: types::ApplSettings,
) -> types::EmptyRes {
    defmt::debug!("Handling set_appl_settings");
    let mut new_config = crate::storage::ApplicationConfig::default();
    new_config.led_blink_duration = rqst.led_blink_duration_ms;

    if crate::storage::APP_CONFIG.set(new_config).await.is_err() {
        defmt::error!("Failed to save app settings");
    }
    types::EmptyRes {}
}

pub(crate) async fn blink_led_n_times(
    context: &mut DispatchContext,
    _header: VarHeader,
    rqst: u8,
) -> types::EmptyRes {
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
    types::EmptyRes {}
}

const DEBOUNCE_TIME_MS: u64 = 20;
pub(crate) async fn button_events_topic_worker(
    button: &Mutex<Input<'static>>,
    sender: Sender<BleWireTxImpl>,
) {
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
            .publish::<topics::ButtonEvents>(
                seq.into(),
                &types::ButtonEvent {
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
pub(crate) async fn start_button_events_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: types::NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_button_events_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::ButtonEvents::TOPIC_KEY);
    // Consume any pending stop signal, these have arrived before we started (we started the task, we are in it,
    // but we haven't responded to the spawn Rpc request yet. So for the client we are still starting...)
    let _ = topic_stop_signal.try_take();
    if sender
        .reply::<endpoints::StartButtonEventsTopic>(header.seq_no, &(().into()))
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

pub(crate) async fn stop_button_events_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::EmptyRes {
    defmt::error!("Handling stop_button_events_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::ButtonEvents::TOPIC_KEY);
    topic_stop_signal.signal(());
    types::EmptyRes {}
}

pub(crate) fn echo(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: types::EchoRequest,
) -> types::EchoResponse {
    defmt::debug!("Handling echo");
    types::EchoResponse { inner: rqst.inner }
}

#[embassy_executor::task]
pub(crate) async fn start_bandwidth_test_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: types::NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_bandwidth_test_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::BandwidthTestTopic::TOPIC_KEY);

    let _ = topic_stop_signal.try_take();
    if sender
        .reply::<endpoints::StartTestTopicBandwidth>(header.seq_no, &(().into()))
        .await
        .is_err()
    {
        defmt::error!("Failed to reply to start_bandwidth_test_topic rpc message");
        return;
    }

    let mut seq = 0u8;
    loop {
        let data = types::BandwidthTestTopicData::new(Instant::now().as_ticks() as u8);
        if sender
            .publish::<topics::BandwidthTestTopic>(seq.into(), &data)
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

pub(crate) async fn stop_bandwidth_test_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::EmptyRes {
    defmt::error!("Handling stop_bandwidth_test_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::BandwidthTestTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    types::EmptyRes {}
}

pub(crate) fn do_nothing(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::BandwidthTestData,
) -> types::EmptyRes {
    defmt::debug!("Handling do_nothing");
    types::EmptyRes {}
}
