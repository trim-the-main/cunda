protocol::define_mayna_metadata! {
    device_type: protocol::devices::tirbod::DEVICE_TYPE,
    protocol_version: protocol::devices::tirbod::v1::RPC_PROTOCOL_VERSION,
}

use embassy_futures::select::{Either, select};
use embassy_time::Instant;
use postcard_rpc::{Topic, header::VarHeader, server::Sender};
use protocol::cunda_common::v1::{endpoints, topics, types};

use crate::{
    ble::BleWireTxImpl,
    rpc::context::{DispatchContext, DispatchSpawnContext},
};

pub(crate) fn get_device_id(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::DeviceId {
    defmt::debug!("Handling get_device_id");
    // TODO: Both hardware revision and serial number should be read or calculated from
    // the persistent storage or eFuse...
    protocol::new_device_id_from_metadata!(hardware_revision: 0, serial_number: 0)
}

pub(crate) async fn get_sys_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> endpoints::SysSettings {
    defmt::debug!("Handling get_sys_settings");
    let mut ret = types::SysSettings::default();
    match crate::storage::SYSTEM_CONFIG.get().await {
        Ok(s) => ret.ble_device_name = s.ble_adv_name,
        _ => {}
    }
    ret
}

pub(crate) async fn set_sys_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: types::SysSettings,
) -> types::EmptyRes {
    defmt::debug!("Handling set_sys_settings");
    let mut new_config = crate::storage::SysConfig::default();
    new_config.ble_adv_name = rqst.ble_device_name;

    if crate::storage::SYSTEM_CONFIG.set(new_config).await.is_err() {
        defmt::error!("Failed to save sys settings");
    }
    types::EmptyRes {}
}

pub(crate) fn sys_ping(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::EmptyRes {
    defmt::debug!("Handling sys_ping");
    types::EmptyRes {}
}

pub(crate) fn sys_stats() -> types::SysStats {
    let memstats = esp_alloc::HEAP.stats();
    types::SysStats {
        cpu_usage: crate::stats::cpu_stats(),
        memory_usage: types::MemoryUsage {
            used: memstats.current_usage as u32,
            total: memstats.size as u32,
        },
        uptime: Instant::now().as_secs() as u32,
    }
}

#[embassy_executor::task]
pub(crate) async fn start_sys_stats_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: types::NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_sys_stats_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::sys::SysStatsTopic::TOPIC_KEY);
    topic_stop_signal.try_take(); // clear the pending stop signals (we haven't responded to the start request yet.)
    let _guard = crate::stats::start_collecting_stats();
    if let Err(err) = sender
        .reply::<endpoints::StartSysStatsTopic>(header.seq_no, &(().into()))
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

        if let Err(err) = sender
            .publish::<topics::sys::SysStatsTopic>(seq.into(), &stats)
            .await
        {
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

pub(crate) fn stop_sys_stats_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::EmptyRes {
    defmt::debug!("Handling stop_sys_stats_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::sys::SysStatsTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    types::EmptyRes {}
}

#[embassy_executor::task]
pub(crate) async fn start_sys_logs_topic(
    context: DispatchSpawnContext,
    header: VarHeader,
    _rqst: types::NoArg,
    sender: Sender<BleWireTxImpl>,
) {
    defmt::debug!("Handling start_sys_logs_topic");
    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::sys::SysLogsTopic::TOPIC_KEY);
    topic_stop_signal.try_take(); // clear the pending stop signals (we haven't responded to the start request yet.)
    if let Err(err) = sender
        .reply::<endpoints::StartSysLogsTopic>(header.seq_no, &(().into()))
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
                let mut log_msg = types::LogMessage::default();
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
            .publish::<topics::sys::SysLogsTopic>(seq.into(), &logs_to_send)
            .await
        {
            defmt::error!("Send error! {}", err);
            break;
        }
        seq = seq.wrapping_add(1);
    }
}

pub(crate) fn stop_sys_logs_topic(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::EmptyRes {
    defmt::debug!("Handling stop_sys_logs_topic");

    let topic_stop_signal = context
        .task_table
        .stop_signal(topics::sys::SysLogsTopic::TOPIC_KEY);
    topic_stop_signal.signal(());
    types::EmptyRes {}
}

pub(crate) async fn get_mtu(
    context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> u16 {
    defmt::debug!("Handling get_mtu");
    context.tx.get_current_mtu().await.unwrap_or(0)
}

pub(super) use crate::rpc::ota::{
    approve_firmware as ota_approve_firmware, factory_reset as ota_factory_reset,
    finalize as ota_finalize, prepare as ota_prepare, transfer_bytes as ota_transfer_bytes,
};
