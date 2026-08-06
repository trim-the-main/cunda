use embassy_futures::select::{Either, select};
use postcard_rpc::{Topic, header::VarHeader, server::Sender};

use protocol::cunda_common::v1::topics::gps as gps_topics;
use protocol::cunda_common::v1::types::RawNmea0183Sentence;
use protocol::cunda_common::v1::{endpoints as common_endpoints, types::GpsDataWire};
use protocol::devices::nokta::v1::types;

use crate::{
    ble::BleWireTxImpl,
    rpc::context::{DispatchContext, DispatchSpawnContext},
};

pub(crate) async fn get_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::NoktaSettings {
    defmt::debug!("Handling get_appl_settings");
    let mut a_settings = types::NoktaSettings::new();
    match crate::storage::APP_CONFIG.get().await {
        Ok(a) => a_settings.led_blink_duration_ms = a.led_blink_duration,
        _ => {}
    }
    a_settings
}

pub(crate) async fn set_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: types::NoktaSettings,
) -> types::EmptyRes {
    defmt::debug!("Handling set_appl_settings");
    let mut new_config = crate::storage::ApplicationConfig::default();
    new_config.led_blink_duration = rqst.led_blink_duration_ms;

    if crate::storage::APP_CONFIG.set(new_config).await.is_err() {
        defmt::error!("Failed to save app settings");
    }
    types::EmptyRes {}
}

#[embassy_executor::task]
pub(crate) async fn start_raw_nmea_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: types::NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_raw_nmea_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(gps_topics::RawNmeaTopic::TOPIC_KEY);
    topic_stop_signal.try_take(); // clear the pending stop signals (we haven't responded to the start request yet.)

    let Ok(mut sub) = context.rt_ctxt.nmea_bcast_channel.subscriber() else {
        defmt::error!("Failed to create nmea subscription");
        if let Err(err) = sender
            .error(
                header.seq_no,
                postcard_rpc::standard_icd::WireError::FailedToSpawn,
            )
            .await
        {
            defmt::error!("Error sending FailedToSpawn error {}", err);
        }
        return;
    };

    if let Err(err) = sender
        .reply::<common_endpoints::StartRawNmeaTopic>(header.seq_no, &(().into()))
        .await
    {
        defmt::error!(
            "Failed to reply to start_raw_nmea_topic rpc message {}",
            err
        );
        return;
    }

    let mut seq = 0u8;
    loop {
        let msg = match select(sub.next_message(), topic_stop_signal.wait()).await {
            Either::First(msg) => msg,
            Either::Second(_) => break,
        };
        match msg {
            embassy_sync::pubsub::WaitResult::Lagged(x) => {
                defmt::warn!("Raw Nmea topic lagged {} sentences", x);
                continue;
            }
            embassy_sync::pubsub::WaitResult::Message(sentence_bytes) => {
                let Ok(sentence) = RawNmea0183Sentence::try_from(sentence_bytes.as_ref()) else {
                    defmt::warn!("Error creating RawNmea0183Sentence");
                    continue;
                };
                if let Err(err) = sender
                    .publish::<gps_topics::RawNmeaTopic>(seq.into(), &sentence)
                    .await
                {
                    defmt::error!("Send error! {}", err);
                    break;
                }
                seq = seq.wrapping_add(1);
            }
        }
    }
}

pub(crate) fn stop_raw_nmea_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::EmptyRes {
    defmt::debug!("Handling stop_raw_nmea_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(gps_topics::RawNmeaTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    types::EmptyRes {}
}

#[embassy_executor::task]
pub(crate) async fn start_parsed_gps_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: types::NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_parsed_gps_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(gps_topics::ParsedGpsTopic::TOPIC_KEY);
    topic_stop_signal.try_take(); // clear the pending stop signals (we haven't responded to the start request yet.)

    let Ok(mut sub) = context.rt_ctxt.gps_broadcast_channel.subscriber() else {
        defmt::error!("Failed to create parsed_gps subscription");
        if let Err(err) = sender
            .error(
                header.seq_no,
                postcard_rpc::standard_icd::WireError::FailedToSpawn,
            )
            .await
        {
            defmt::error!("Error sending FailedToSpawn error {}", err);
        }
        return;
    };

    if let Err(err) = sender
        .reply::<common_endpoints::StartParsedGpsTopic>(header.seq_no, &(().into()))
        .await
    {
        defmt::error!(
            "Failed to reply to start_parsed_gps_topic rpc message {}",
            err
        );
        return;
    }

    let mut seq = 0u8;
    loop {
        let msg = match select(sub.next_message(), topic_stop_signal.wait()).await {
            Either::First(msg) => msg,
            Either::Second(_) => break,
        };

        match msg {
            embassy_sync::pubsub::WaitResult::Lagged(x) => {
                defmt::warn!("Parsed gps topic lagged {} broadcasts", x);
                continue;
            }
            embassy_sync::pubsub::WaitResult::Message(gps_data) => {
                let gps_data_wire: GpsDataWire = gps_data.into();
                if let Err(err) = sender
                    .publish::<gps_topics::ParsedGpsTopic>(seq.into(), &gps_data_wire)
                    .await
                {
                    defmt::error!("Send error! {}", err);
                    break;
                }
                seq = seq.wrapping_add(1);
            }
        }
    }
}

pub(crate) fn stop_parsed_gps_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::EmptyRes {
    defmt::debug!("Handling stop_parsed_gps_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(gps_topics::ParsedGpsTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    types::EmptyRes {}
}
