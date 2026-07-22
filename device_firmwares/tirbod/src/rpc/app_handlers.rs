use postcard_rpc::header::VarHeader;

use protocol::devices::tirbod::v1::types;

use crate::rpc::context::DispatchContext;

pub(crate) async fn get_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::NoArg,
) -> types::TirbodSettings {
    defmt::debug!("Handling get_appl_settings");
    let a_settings = types::TirbodSettings::new();
    match crate::storage::APP_CONFIG.get().await {
        Ok(_a) => {}
        _ => {}
    }
    a_settings
}

pub(crate) async fn set_appl_settings(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: types::TirbodSettings,
) -> types::EmptyRes {
    defmt::debug!("Handling set_appl_settings");
    let new_config = crate::storage::ApplicationConfig::default();

    if crate::storage::APP_CONFIG.set(new_config).await.is_err() {
        defmt::error!("Failed to save app settings");
    }
    types::EmptyRes {}
}
