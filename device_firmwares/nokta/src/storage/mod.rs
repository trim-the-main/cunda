mod data;
mod disk;
mod shared_async_flash;
mod value;
pub use data::app_config::{APP_CONFIG, ApplicationConfig};
pub use data::sys_config::{SYSTEM_CONFIG, SysConfig};
pub use disk::init;
pub mod ota;
