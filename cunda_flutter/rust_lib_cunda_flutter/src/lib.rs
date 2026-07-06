pub mod cunda_device_base;
pub mod defmt_log_translation;
pub mod devices;
mod frb_generated;
pub mod frb_mirrors;
pub mod log_decoder;
pub mod rpc;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
    #[cfg(target_os = "linux")]
    env_logger::builder().format_timestamp_micros().init();
}
