use postcard_rpc::header::VarHeader;

use protocol::devices::nokta::v1::types;

use crate::rpc::context::DispatchContext;

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
