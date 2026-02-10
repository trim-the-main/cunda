use minicbor::{CborLen, Decode, Encode};

use crate::storage::{
    data::DType,
    value::{CachedDiskValue, DiskValueTrait},
};

pub static APP_CONFIG: CachedDiskValue<ApplicationConfig> = CachedDiskValue::new();

#[derive(Clone, Encode, Decode, CborLen, defmt::Format)]
pub struct ApplicationConfig {
    #[n(0)]
    pub led_blink_duration: u32,
}
impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            led_blink_duration: 100,
        }
    }
}

impl DiskValueTrait for ApplicationConfig {
    const KEY: DType = DType::ApplicationConfig;
}
