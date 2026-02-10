use core::str::FromStr;

use minicbor::{CborLen, Decode, Encode};

use crate::storage::{
    data::DType,
    value::{CachedDiskValue, DiskValueTrait},
};

pub static SYSTEM_CONFIG: CachedDiskValue<SysConfig> = CachedDiskValue::new();

#[derive(Clone, Encode, Decode, CborLen, defmt::Format)]
pub struct SysConfig {
    #[cbor(n(0), with = "minicbor_adapters")]
    pub ble_adv_name: heapless::String<20>,
}
impl Default for SysConfig {
    fn default() -> Self {
        Self {
            ble_adv_name: heapless::String::from_str("Cunda").unwrap(),
        }
    }
}
impl DiskValueTrait for SysConfig {
    const KEY: DType = DType::SysConfig;
}
