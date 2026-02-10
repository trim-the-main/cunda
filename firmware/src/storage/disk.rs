extern crate alloc;

use alloc::vec::Vec;
use core::ops::Range;

use embassy_embedded_hal::adapter::BlockingAsync;
use esp_bootloader_esp_idf::partitions;
use esp_storage::{FlashStorage, FlashStorageError};
use maitake_sync::Mutex;
use sequential_storage::{
    cache::HeapPageStateCache,
    map::{MapConfig, MapStorage, SerializationError, Value},
};

use crate::storage::data::DType;

pub(super) static DISK: Mutex<Option<KvDisk<'static>>> = Mutex::new(None);

pub async fn init(flash: FlashStorage<'static>) {
    let mut d = DISK.lock().await;
    *d = Some(KvDisk::new(flash))
}

#[derive(Debug)]
pub enum DiskError {
    ErrorReadingPartitionTable,
    NoKVPartition,
    SeqMapError(sequential_storage::Error<FlashStorageError>),
}

pub(super) struct KvDisk<'d> {
    ms: MapStorage<DType, BlockingAsync<FlashStorage<'d>>, HeapPageStateCache>,
    buffer: Vec<u8>,
}

impl<'d> KvDisk<'d> {
    const BUFFER_SIZE: usize = 256;

    pub(super) fn new(mut flash: FlashStorage<'d>) -> Self {
        let kv_partition_range = find_kv_partition(&mut flash).unwrap();
        let page_count = kv_partition_range.len() / (FlashStorage::SECTOR_SIZE as usize);

        let mut buffer = Vec::with_capacity(Self::BUFFER_SIZE);
        buffer.resize(Self::BUFFER_SIZE, 0u8);
        Self {
            ms: MapStorage::new(
                BlockingAsync::new(flash),
                MapConfig::new(kv_partition_range),
                HeapPageStateCache::new(page_count),
            ),
            buffer,
        }
    }

    pub(super) async fn get<V>(&mut self, data_type: DType) -> Result<Option<V>, DiskError>
    where
        for<'a> V: Value<'a>,
    {
        let res: Option<V> = self
            .ms
            .fetch_item(&mut self.buffer, &data_type)
            .await
            .map_err(DiskError::SeqMapError)?;
        Ok(res)
    }

    pub(super) async fn put<V>(&mut self, data_type: DType, new_value: &V) -> Result<(), DiskError>
    where
        for<'a> V: Value<'a>,
    {
        let res = self
            .ms
            .store_item(&mut self.buffer, &data_type, new_value)
            .await;
        match res {
            Ok(_) => {}
            Err(ref err) => match err {
                sequential_storage::Error::Storage { value: serr } => {
                    defmt::error!("Storage {}", serr)
                }
                sequential_storage::Error::FullStorage => defmt::error!("FullStorage "),
                sequential_storage::Error::Corrupted {} => defmt::error!("Corrupted "),
                sequential_storage::Error::LogicBug {} => defmt::error!("LogicBug "),
                sequential_storage::Error::BufferTooBig => defmt::error!("BufferTooBig "),
                sequential_storage::Error::BufferTooSmall(_) => defmt::error!("BufferTooSmall"),
                sequential_storage::Error::SerializationError(sererr) => {
                    defmt::error!("SerializationError {}", sererr);
                }
                sequential_storage::Error::ItemTooBig => defmt::error!("ItemTooBig"),
                _ => todo!(),
            },
        }
        res.map_err(DiskError::SeqMapError)
    }
}

impl DType {
    fn discriminant(&self) -> u32 {
        // SAFETY: Because `Self` is marked `repr(u32)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u32` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u32>() }
    }
}

impl sequential_storage::map::Key for DType {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        let len = size_of::<Self>();
        if buffer.len() < len {
            return Err(SerializationError::BufferTooSmall);
        }
        buffer[..len].copy_from_slice(&self.discriminant().to_le_bytes());
        Ok(len)
    }

    fn deserialize_from(buffer: &[u8]) -> Result<(Self, usize), SerializationError> {
        let len = size_of::<Self>();
        if buffer.len() < len {
            return Err(SerializationError::BufferTooSmall);
        }

        let discriminant: u32 = u32::from_le_bytes(buffer[..len].try_into().unwrap());
        let dtype = if discriminant >= DType::UnknownType.discriminant() {
            DType::UnknownType
        } else {
            // SAFETY: It's a u32 within the range of DType enum representation thanks to
            // `repr(C)` and Invalid being the last
            unsafe { core::mem::transmute::<u32, DType>(discriminant) }
        };
        Ok((dtype, len))
    }
}

fn find_kv_partition(flash: &mut FlashStorage) -> Result<Range<u32>, DiskError> {
    const KV_PARTITION_MAGIC: u8 = 0x06;
    use DiskError::*;
    let mut buffer = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = partitions::read_partition_table(flash, &mut buffer).unwrap();

    for entry in pt.iter() {
        if entry.raw_subtype() == KV_PARTITION_MAGIC {
            let begin = entry.offset();
            let end = begin + entry.len();
            return Ok(begin..end);
        }
    }
    Err(NoKVPartition)
}
