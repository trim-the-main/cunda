use minicbor::{CborLen, Decode, Encode};

use crate::storage::{
    data::DType,
    value::{CachedDiskValue, KvStoreEntry},
};

pub static APP_CONFIG: CachedDiskValue<ApplicationConfig> = CachedDiskValue::new();

#[derive(Clone, Encode, Decode, CborLen, defmt::Format)]
pub struct ApplicationConfig {}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {}
    }
}

impl KvStoreEntry for ApplicationConfig {
    const KEY: DType = DType::ApplicationConfig;
}
