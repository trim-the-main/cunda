use core::num::NonZeroU32;
use maitake_sync::Mutex;

use embedded_storage::{ReadStorage, Storage};

use embedded_storage::nor_flash::{ErrorType, MultiwriteNorFlash, NorFlash, ReadNorFlash};
use embedded_storage_async::nor_flash::{
    MultiwriteNorFlash as AsyncMultiwriteNorFlash, NorFlash as AsyncNorFlash,
    ReadNorFlash as AsyncReadNorFlash,
};

#[derive(Debug)]
pub struct SharedAsyncFlashRegion<'a, BlockingFlashStorage> {
    m_flash: &'a Mutex<BlockingFlashStorage>,
    base_offset: u32,
    capacity: NonZeroU32,
}

impl<'a, BlockingFlash> SharedAsyncFlashRegion<'a, BlockingFlash> {
    pub fn try_new(
        m_flash: &'a Mutex<BlockingFlash>,
        base_offset: u32,
        capacity: u32,
    ) -> Option<SharedAsyncFlashRegion<'a, BlockingFlash>> {
        let capacity = NonZeroU32::new(capacity)?;
        Some(Self {
            m_flash,
            base_offset,
            capacity,
        })
    }

    // split_at is the byte number in the region (not the base_offset)
    #[allow(dead_code)]
    pub fn split(self, split_at: u32) -> (Option<Self>, Option<Self>) {
        if split_at >= self.capacity.get() {
            return (Some(self), None);
        }
        if split_at == 0 {
            return (None, Some(self));
        }

        let left = Self {
            m_flash: self.m_flash,
            base_offset: self.base_offset,
            capacity: NonZeroU32::new(split_at).unwrap(), // We checked that split_at is larger than zero
        };

        let right = Self {
            m_flash: self.m_flash,
            base_offset: self.base_offset + split_at,
            capacity: NonZeroU32::new(self.capacity.get() - split_at).unwrap(), // split_at is smaller than capacity
        };
        (Some(left), Some(right))
    }
}

impl<'a, BlockingFlash: ReadStorage> SharedAsyncFlashRegion<'a, BlockingFlash> {
    pub async fn with_entire_flash_storage<F, R>(&self, cb: F) -> R
    where
        F: for<'storage> FnOnce(&'storage mut BlockingFlash) -> R,
    {
        let mut flash = self.m_flash.lock().await;
        cb(&mut *flash)
    }
}

impl<'a, F> ErrorType for SharedAsyncFlashRegion<'a, F>
where
    F: ErrorType,
{
    type Error = <F as ErrorType>::Error;
}

impl<'a, F> AsyncReadNorFlash for SharedAsyncFlashRegion<'a, F>
where
    F: ReadNorFlash,
{
    const READ_SIZE: usize = <F as ReadNorFlash>::READ_SIZE;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let mut f = self.m_flash.lock().await;
        f.read(offset + self.base_offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.capacity.get().try_into().unwrap()
    }
}

impl<'a, F> AsyncNorFlash for SharedAsyncFlashRegion<'a, F>
where
    F: NorFlash,
{
    const WRITE_SIZE: usize = <F as NorFlash>::WRITE_SIZE;

    const ERASE_SIZE: usize = <F as NorFlash>::ERASE_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let mut f = self.m_flash.lock().await;

        defmt::debug!(
            "Flash erase from {} to {}",
            from + self.base_offset,
            to + self.base_offset
        );

        let mut buf = [0u8; 256];

        let mut need_erase = false;
        for offset in (from..to).step_by(256) {
            f.read(self.base_offset + offset, &mut buf)?;
            if !buf.iter().all(|&b| b == 0xFF) {
                need_erase = true;
                break;
            }
        }
        if need_erase {
            defmt::debug!("No need to erase from: {} to: {}", from, to);
            f.erase(from + self.base_offset, to + self.base_offset)
        } else {
            defmt::info!(
                "No need to erase from: {} to: {}, it's already all 1s",
                from,
                to
            );
            Ok(())
        }
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut f = self.m_flash.lock().await;
        f.write(offset + self.base_offset, bytes)
    }
}

// These traits are taken from the HEAD of embedded-storage-async git repo
#[allow(dead_code)]
pub trait AsyncReadStorage {
    type Error;
    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error>;
    fn capacity(&self) -> usize;
}

#[allow(dead_code)]
pub trait AsyncStorage: AsyncReadStorage {
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error>;
}

impl<'a, F> AsyncReadStorage for SharedAsyncFlashRegion<'a, F>
where
    F: ReadStorage,
{
    type Error = <F as ReadStorage>::Error;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let mut f = self.m_flash.lock().await;
        f.read(offset + self.base_offset, bytes)
    }

    fn capacity(&self) -> usize {
        self.capacity.get().try_into().unwrap()
    }
}

impl<'a, F> AsyncStorage for SharedAsyncFlashRegion<'a, F>
where
    F: Storage,
{
    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut f = self.m_flash.lock().await;
        f.write(offset + self.base_offset, bytes)
    }
}
impl<'a, F> AsyncMultiwriteNorFlash for SharedAsyncFlashRegion<'a, F> where F: MultiwriteNorFlash {}
