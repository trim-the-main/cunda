extern crate alloc;

use alloc::vec::Vec;
use core::ops::Range;

use esp_bootloader_esp_idf::partitions;
use esp_storage::{FlashStorage, FlashStorageError};
use maitake_sync::Mutex;
use sequential_storage::{
    cache::HeapPageStateCache,
    map::{MapConfig, MapStorage, SerializationError, Value},
};

use crate::storage::{data::DType, ota::OtaState, shared_async_flash::SharedAsyncFlashRegion};

const DATA_PARTITION_TYPE: u8 = 0x01;
const KV_PARTITION_SUBTYPE: u8 = 0x06;
pub(super) static KV_STORE: Mutex<Option<KvDisk<'static>>> = Mutex::new(None);

// The init function here is a bit convoluted because of the different traits each operation require.
// We take this `FlashStorage<'static>` which is mainly a blocking type. The KV store needs an async
// interface and OTA needs a reference to the entire flash region etc. So we initially create the OTA
// object from esp_bootloader_esp_idf package temporarily. This reads the partition table finds the ota
// data partition, reads and parses it. We only keep the "next_app_partition" (where should we write the
// new bytes) and the ota state. Later we put the flash storage behind an async mutex. That way we can
// share the flash storage between different async tasks. We initialize two global objects with a reference
// to this mutex and the ranges of distinct partitions: KV store and OTA objects. OTA is just the slot to
// write the incoming bytes from the postcard_rpc firmware upgrade.
pub async fn init(
    mut flash: FlashStorage<'static>,
    sha: &'static Mutex<esp_hal::sha::Sha<'static>>,
) {
    let mut buffer = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
    let (next_app_partition, ota_state) = {
        let mut ota_updater =
            esp_bootloader_esp_idf::ota_updater::OtaUpdater::new(&mut flash, &mut buffer)
                .expect("Failed to initialize OtaUpdater");
        let ota_state = match ota_updater.current_ota_state() {
            Ok(esp_bootloader_esp_idf::ota::OtaImageState::New) => panic!(
                "Bootloader must change new state to pendingVerify but it didn't. Bootloader may not have auto-rollback support"
            ),
            Ok(esp_bootloader_esp_idf::ota::OtaImageState::PendingVerify) => {
                defmt::info!("Bootloader selected an Ota image in pending verify state");
                OtaState::PendingVerify
            }
            Ok(esp_bootloader_esp_idf::ota::OtaImageState::Valid) => {
                defmt::info!("Bootloader selected an Ota image in valid state");
                OtaState::Ready
            }
            Ok(esp_bootloader_esp_idf::ota::OtaImageState::Invalid) => {
                panic!("Bootloader must not boot an invalid app")
            }
            Ok(esp_bootloader_esp_idf::ota::OtaImageState::Aborted) => {
                panic!("Bootloader must not boot an invalid app")
            }
            Ok(esp_bootloader_esp_idf::ota::OtaImageState::Undefined) => {
                defmt::info!("Bootloader selected an Ota image with undefined state");
                OtaState::Ready
            }
            Err(partitions::Error::InvalidState) => {
                defmt::info!("Bootloader loaded factory partition. OTA data is erased");
                OtaState::Ready
            }
            Err(err) => {
                panic!("Failed to read OTA data partition {}", err);
            }
        };
        (
            ota_updater
                .next_partition()
                .expect("Storage error reading the OTA partition")
                .1,
            ota_state,
        )
    };

    static DISK: static_cell::StaticCell<Mutex<FlashStorage<'static>>> =
        static_cell::StaticCell::new();
    let disk = &*DISK.init(Mutex::new(flash));

    // Read the partition table, find the kv partition and the next ota partition ranges and initialize the
    // kv store and OTA globals behind the mutexes. These will need the reference to the disk which is the
    // underlying flash behind its own mutex.
    let mut kv_partition_range = None;
    let mut ota_new_firmware_range = None;
    {
        let mut flash = disk.lock().await;

        let pt = esp_bootloader_esp_idf::partitions::read_partition_table(&mut *flash, &mut buffer)
            .unwrap();

        for entry in pt.iter() {
            let begin = entry.offset();
            let end = begin + entry.len();
            if entry.raw_type() == DATA_PARTITION_TYPE
                && entry.raw_subtype() == KV_PARTITION_SUBTYPE
            {
                kv_partition_range = Some(begin..end);
            } else if matches!(
                entry.partition_type(),
                esp_bootloader_esp_idf::partitions::PartitionType::App(subtype) if subtype == next_app_partition
            ) {
                defmt::info!("OTA slot (the passive partition) {:X} {:X}", begin, end);
                ota_new_firmware_range = Some(begin..end);
            }
        }
    }

    {
        let mut d = KV_STORE.lock().await;
        *d = Some(KvDisk::new(
            disk,
            kv_partition_range.expect("Did not find kv partition"),
        ));
    }

    if let Some(ota_new_firmware_range) = ota_new_firmware_range {
        let mut d = crate::storage::ota::OTA.lock().await;
        *d = Some(crate::storage::ota::CundaOta::new(
            ota_state,
            SharedAsyncFlashRegion::try_new(
                disk,
                ota_new_firmware_range.start,
                ota_new_firmware_range.end - ota_new_firmware_range.start,
            )
            .expect("ota partition cannot have zero size"),
            next_app_partition,
            sha,
        ));
    }
}

#[derive(Debug)]
pub enum DiskError {
    ErrorReadingPartitionTable,
    NoKVPartition,
    SeqMapError(sequential_storage::Error<FlashStorageError>),
}

pub(super) struct KvDisk<'d> {
    ms: MapStorage<DType, SharedAsyncFlashRegion<'d, FlashStorage<'d>>, HeapPageStateCache>,
    buffer: Vec<u8>,
}

impl<'d> KvDisk<'d> {
    const BUFFER_SIZE: usize = 256;

    pub(super) fn new(flash: &'d Mutex<FlashStorage<'d>>, kv_partition_range: Range<u32>) -> Self {
        let size = kv_partition_range.len();
        let page_count = size / (FlashStorage::SECTOR_SIZE as usize);

        let mut buffer = Vec::with_capacity(Self::BUFFER_SIZE);
        buffer.resize(Self::BUFFER_SIZE, 0u8);
        Self {
            ms: MapStorage::new(
                SharedAsyncFlashRegion::try_new(flash, kv_partition_range.start, size as u32)
                    .expect("Failed to initialize KV store"),
                MapConfig::new(0..size as u32),
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
