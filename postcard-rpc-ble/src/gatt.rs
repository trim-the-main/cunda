use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use trouble_host::prelude::*;

pub const BLE_MTU: usize = 255;
pub const GATT_OVERHEAD: usize = 7;

#[gatt_service(uuid = "408813DF-5DD4-1F87-EC11-CDB001100000")]
pub(crate) struct RpcService {
    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "rx_not_acked", read, value = "rx buffer with no ack")]
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100001", write_without_response)]
    pub rx_not_acked: BytesCh,

    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "rx_acked", read, value = "rx buffer with acks")]
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100002", write)]
    pub rx_acked: BytesCh,

    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "tx_not_acked", read, value = "tx buffer with no ack")]
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100003", notify)]
    pub tx_not_acked: BytesCh,

    #[descriptor(uuid = descriptors::MEASUREMENT_DESCRIPTION, name = "tx_acked", read, value = "tx buffer with acks")]
    #[characteristic(uuid = "408813df-5dd4-1f87-ec11-cdb001100004", indicate)]
    pub tx_acked: BytesCh,
}

#[gatt_server(mutex_type = CriticalSectionRawMutex)]
pub(crate) struct GattServerRpc {
    pub rpc_service: RpcService,
}

pub struct BytesCh {
    data: [u8; Self::MAX_SIZE],
    used_len: usize,
}

impl BytesCh {
    pub(crate) const MAX_SIZE: usize = BLE_MTU - GATT_OVERHEAD;

    pub(crate) fn try_from_slice(src: &[u8]) -> Option<Self> {
        let mut retval: BytesCh = Default::default();
        if src.len() > retval.data.len() {
            return None;
        }
        retval.data[..src.len()].copy_from_slice(src);
        retval.used_len = src.len();
        Some(retval)
    }
}

impl Default for BytesCh {
    fn default() -> Self {
        Self {
            data: [0u8; Self::MAX_SIZE],
            used_len: 0,
        }
    }
}

impl AsGatt for BytesCh {
    const MIN_SIZE: usize = 0;
    const MAX_SIZE: usize = Self::MAX_SIZE;

    fn as_gatt(&self) -> &[u8] {
        &self.data[0..self.used_len]
    }
}

impl FromGatt for BytesCh {
    fn from_gatt(data: &[u8]) -> Result<Self, trouble_host::types::gatt_traits::FromGattError> {
        let mut msg = [0u8; Self::MAX_SIZE];
        msg[..data.len()].copy_from_slice(data);
        Ok(Self {
            data: msg,
            used_len: data.len(),
        })
    }
}

impl defmt::Format for BytesCh {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{=[u8]}", self.data[..self.used_len]);
    }
}
