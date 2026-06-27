pub mod defmt_log_translation;
mod frb_generated;
pub mod rpc;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
    #[cfg(target_os = "linux")]
    env_logger::builder().format_timestamp_micros().init();
}
