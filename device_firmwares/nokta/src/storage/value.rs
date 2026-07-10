use maitake_sync::{Mutex, WaitQueue};
use minicbor::{CborLen, Decode, Encode};
use sequential_storage::map::{SerializationError, Value};

use crate::storage::{
    data::DType,
    disk::{DiskError, KV_STORE},
};

// marker trait: use sequential_storage map with cbor encoding
pub trait KvStoreEntry:
    Clone + Default + Encode<()> + for<'a> Decode<'a, ()> + CborLen<()> + defmt::Format
{
    const KEY: DType;
}

// I wish there was a feature flag in sequential_storage to implement
// the Value trait (serialization, deserialization) for cbor types.
// Because of rust orphan rules we need a wrapper type.
#[derive(Default)]
pub struct DiskValue<T: KvStoreEntry>(T);

// This is the main interface
// it caches the setting in RAM
// allows for listening for the changes
pub struct CachedDiskValue<T: KvStoreEntry> {
    mtx: Mutex<Option<DiskValue<T>>>,
    wq: WaitQueue,
}

impl<T: KvStoreEntry> CachedDiskValue<T> {
    pub const fn new() -> Self {
        Self {
            mtx: Mutex::new(None),
            wq: WaitQueue::new(),
        }
    }

    pub async fn get(&self) -> Result<T, DiskError> {
        let mut value = self.mtx.lock().await;

        match &*value {
            Some(v) => return Ok(v.0.clone()),
            None => {
                // data is not initialized, try to read from flash and cache
                let mut guard = KV_STORE.lock().await;
                let disk = guard.as_mut().expect("Disk access before initialization");
                *value = match disk.get::<DiskValue<T>>(T::KEY).await? {
                    Some(v) => Some(v),
                    None => Some(Default::default()),
                };
                return Ok(value.as_ref().unwrap().0.clone());
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
        defmt::debug!("Setting new value {:?}", new_value);
        let mut value = self.mtx.lock().await;
        // if Some(new_value) == *data {
        //     return Ok(()); // new value is the same as old
        // }

        // write it to flash first, then notify listeners
        let new_value = DiskValue::new(new_value);
        {
            let mut guard = KV_STORE.lock().await;
            let disk = guard.as_mut().expect("Disk access before initialization");
            disk.put::<DiskValue<T>>(T::KEY, &new_value).await?;
        }

        *value = Some(new_value);
        self.wq.wake_all();
        Ok(())
    }

    pub async fn listen(&self) {
        self.wq
            .wait()
            .await
            .expect("Waitqueue is closed unexpectedly");
    }
}

impl<T: KvStoreEntry> DiskValue<T> {
    pub fn new(val: T) -> Self {
        Self(val)
    }
}

impl<'b, T: KvStoreEntry> Value<'b> for DiskValue<T> {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        minicbor::encode(&self.0, buffer).map_err(|_| SerializationError::BufferTooSmall)?;
        Ok(self.0.cbor_len(&mut ()))
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
