extern crate alloc;

use alloc::vec::Vec;
use core::{ops::Range, str::FromStr};

use embassy_embedded_hal::adapter::BlockingAsync;
use esp_bootloader_esp_idf::partitions;
use esp_storage::{FlashStorage, FlashStorageError};
use maitake_sync::{Mutex, WaitQueue};
use minicbor::{CborLen, Decode, Encode};
use sequential_storage::{
    cache::HeapPageStateCache,
    map::{MapConfig, MapStorage, SerializationError, Value},
};

static DISK: Mutex<Option<Disk<'static>>> = Mutex::new(None);
pub async fn init(flash: FlashStorage<'static>) {
    let mut d = DISK.lock().await;
    *d = Some(Disk::new(flash))
}

#[derive(Debug)]
pub enum DiskError {
    ErrorReadingPartitionTable,
    NoKVPartition,
    SeqMapError(sequential_storage::Error<FlashStorageError>),
}

const KV_PARTITION_MAGIC: u8 = 0x06;
fn find_kv_partition(flash: &mut FlashStorage) -> Result<Range<u32>, DiskError> {
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

struct Disk<'d> {
    ms: MapStorage<DType, BlockingAsync<FlashStorage<'d>>, HeapPageStateCache>,
    buffer: Vec<u8>,
}

impl<'d> Disk<'d> {
    const BUFFER_SIZE: usize = 256;

    pub fn new(mut flash: FlashStorage<'d>) -> Self {
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

    pub async fn get<V>(&mut self, data_type: DType) -> Result<Option<V>, DiskError>
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

    pub async fn put<V>(&mut self, data_type: DType, new_value: &V) -> Result<(), DiskError>
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

// Do not remove any item, this list is append only for version compatibility
// reasons. A newer firmware should be able to deserialize a u32 to this type.
// An older firmware should also be able to deserialize a u32, mapping any new
// variant to UnknownType variant.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DType {
    SysConfig = 0,
    ApplicationConfig,
    PanicMessage,

    // Any u32 bigger than or equal to this is unknown to us. We may find such
    // values in case of a version downgrade. This is the only field whose value
    // is okay to to move. It has to be at the bottom of the list.
    UnknownType,
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

#[derive(Clone, Encode, Decode, CborLen)]
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

#[derive(Clone, Encode, Decode, CborLen)]
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
// marker trait: use sequential_storage map with cbor encoding
pub trait DiskValueTrait:
    Clone + Default + Encode<()> + for<'a> Decode<'a, ()> + CborLen<()>
{
    const KEY: DType;
}

#[derive(Default)]
pub struct DiskValue<T: DiskValueTrait> {
    val: T,
}

pub struct CacheInner<T: DiskValueTrait> {
    inner_disk_value: Option<DiskValue<T>>,
    listeners: WaitQueue,
}

impl<T: DiskValueTrait> CacheInner<T> {
    pub const fn new() -> Self {
        Self {
            inner_disk_value: None,
            listeners: WaitQueue::new(),
        }
    }
}

pub struct CachedDiskValue<T: DiskValueTrait> {
    mutex: Mutex<CacheInner<T>>,
}

impl<T: DiskValueTrait> CachedDiskValue<T> {
    pub const fn new() -> Self {
        Self {
            mutex: Mutex::new(CacheInner::new()),
        }
    }

    pub async fn get(&self) -> Result<T, DiskError> {
        let mut cache = self.mutex.lock().await;

        match cache.inner_disk_value {
            Some(ref v) => return Ok(v.val.clone()),
            None => {
                // data is not initialized, try to read from flash and cache
                let mut guard = DISK.lock().await;
                let disk = guard.as_mut().expect("Disk access before initialization");
                cache.inner_disk_value = match disk.get::<DiskValue<T>>(T::KEY).await? {
                    Some(v) => Some(v),
                    None => Some(Default::default()),
                };
                return Ok(cache.inner_disk_value.as_ref().unwrap().val.clone());
            }
        }
    }
    pub async fn get_or_default(&self) -> T {
        match self.get().await {
            Ok(val) => val,
            _ => Default::default(),
        }
    }

    pub async fn set(&self, new_value: T) -> Result<(), DiskError> {
        let mut cache = self.mutex.lock().await;
        // if Some(new_value) == *data {
        //     return Ok(()); // new value is the same as old
        // }

        // write it to flash first, then notify listeners
        let new_value = DiskValue::new(new_value);
        {
            let mut guard = DISK.lock().await;
            let disk = guard.as_mut().expect("Disk access before initialization");
            disk.put::<DiskValue<T>>(T::KEY, &new_value).await?;
        }

        cache.inner_disk_value = Some(new_value);
        cache.listeners.wake_all();
        Ok(())
    }
}

pub static SYSTEM_CONFIG: CachedDiskValue<SysConfig> = CachedDiskValue::new();
pub static APP_CONFIG: CachedDiskValue<ApplicationConfig> = CachedDiskValue::new();

impl<T: DiskValueTrait> DiskValue<T> {
    pub fn new(val: T) -> Self {
        Self { val }
    }
}

impl<'b, T: DiskValueTrait> Value<'b> for DiskValue<T> {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        minicbor::encode(&self.val, buffer).map_err(|_| SerializationError::BufferTooSmall)?;
        Ok(self.val.cbor_len(&mut ()))
    }

    fn deserialize_from(buffer: &'b [u8]) -> Result<(Self, usize), SerializationError>
    where
        Self: Sized,
    {
        let val =
            minicbor::decode::<'b, T>(buffer).map_err(|_err| SerializationError::InvalidFormat)?;
        Ok((Self::new(val), buffer.len()))
    }
}
