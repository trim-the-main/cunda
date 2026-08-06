use embedded_storage::nor_flash::NorFlash;
use esp_bootloader_esp_idf::{
    ota,
    partitions::{self, AppPartitionSubType},
};
use esp_hal::sha;
use esp_storage::{FlashStorage, FlashStorageError};
use maitake_sync::Mutex;

use crate::storage::shared_async_flash::SharedAsyncFlashRegion;
use embedded_storage_async::nor_flash::{
    NorFlash as AsyncNorFlash, ReadNorFlash as AsyncReadNorFlash,
};

extern crate alloc;

pub(crate) static OTA: Mutex<Option<CundaOta>> = Mutex::new(None);

// Naming things get hard, especially when you need to wrap the already wrapped struct.
// esp_bootloader_esp_idf package has ota, ota_updater now I need to name one more.
// This struct is initialized at the beginning with disk::init(). The main purpose is
// to have a handle to the flash storage to be able to write the new app in a firmware
// update. During the write operation, the `ota_state` is changed to WritingNewFirmware
// with the sha256 sum. At the end we verify the new partition with this checksum (hence
// the reference to Sha object, for hardware acceleration).
pub(crate) struct CundaOta {
    ota_state: OtaState,
    new_app_slot: SharedAsyncFlashRegion<'static, FlashStorage<'static>>,
    new_app_type: AppPartitionSubType,
    sha: &'static Mutex<sha::Sha<'static>>,
}

#[derive(Debug, Clone)]
pub(crate) enum OtaState {
    PendingVerify,
    Ready,
    WritingNewFirmware(FirmwareVerificationData),
}

#[derive(Debug, Clone)]
pub struct FirmwareVerificationData {
    size: u32,
    sha256digest: [u8; 32],
}

impl CundaOta {
    pub const fn new(
        ota_state: OtaState,
        new_app_slot: SharedAsyncFlashRegion<'static, FlashStorage<'static>>,
        new_app_type: AppPartitionSubType,
        sha: &'static Mutex<sha::Sha<'static>>,
    ) -> Self {
        Self {
            ota_state,
            new_app_slot,
            new_app_type,
            sha,
        }
    }

    pub async fn prepare(
        &mut self,
        size: u32,
        sha256digest: [u8; 32],
    ) -> Result<(), FlashStorageError> {
        defmt::info!(
            "Ota update prepare: size: {}, sha256: {}",
            size,
            sha256digest
        );

        // In order to prevent the unnecessary erase operations, we issue them one by one and
        // read from flash before erasing. If data read is all 1s erasing is unnecessary, so we
        // don't do it.
        for offset in (0u32..self.new_app_slot.capacity() as _)
            .step_by(<FlashStorage as NorFlash>::ERASE_SIZE)
        {
            self.new_app_slot
                .erase(
                    offset,
                    offset + <FlashStorage as NorFlash>::ERASE_SIZE as u32,
                )
                .await?;
        }
        self.ota_state =
            OtaState::WritingNewFirmware(FirmwareVerificationData { size, sha256digest });
        defmt::info!("Ready for receiving ota bytes",);
        Ok(())
    }

    pub async fn write_bytes(
        &mut self,
        offset: u32,
        bytes: &[u8],
    ) -> Result<(), FlashStorageError> {
        match self.new_app_slot.write(offset, bytes).await {
            Ok(_) => Ok(()),
            Err(err) => {
                defmt::warn!(
                    "Ota write error offset: {} len: {} err: {}",
                    offset,
                    bytes.len(),
                    err
                );
                Err(err)
            }
        }
    }

    pub async fn verify_new_app_transferred_correctly(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(), ()> {
        defmt::info!("Ota verification");
        let OtaState::WritingNewFirmware(FirmwareVerificationData { size, sha256digest }) =
            self.ota_state
        else {
            return Err(());
        };
        defmt::info!(
            "Ota verification: checking with size {} sha {}",
            size,
            sha256digest
        );
        let mut sha = self.sha.lock().await;
        let mut output = [0u8; 32];

        let mut hasher = sha.start::<sha::Sha256>();
        let mut offset = 0u32;
        while offset < size {
            defmt::debug!("Ota verification: reading bytes at {}", offset);
            self.new_app_slot
                .read(offset, buffer)
                .await
                .map_err(|e| defmt::error!("Ota verification flash error: {}", e))?;

            let mut buffer_view = &buffer[..buffer.len().min((size - offset) as _)];

            defmt::debug!(
                "Ota verification: reading bytes at {} size {}",
                offset,
                buffer_view.len()
            );
            offset += buffer_view.len() as u32;
            while !buffer_view.is_empty() {
                // SAFETY: it's okay to unwrap infallible
                buffer_view = nb::block!(hasher.update(buffer_view)).unwrap();
            }
        }

        nb::block!(hasher.finish(output.as_mut_slice())).unwrap();

        defmt::info!("Ota verification: calculated sha256 {}", output);
        if output == sha256digest {
            Ok(())
        } else {
            defmt::warn!(
                "Ota calculated sha256 {} did not match the one given in prepare {}",
                output,
                sha256digest
            );
            Err(())
        }
    }
    pub async fn mark_new_firmware_for_boot(&self) -> Result<(), ()> {
        self.set_partition_for_boot(self.new_app_type).await
    }

    pub async fn mark_factory_for_boot(&self) -> Result<(), ()> {
        self.set_partition_for_boot(AppPartitionSubType::Factory)
            .await
    }

    pub async fn set_partition_for_boot(&self, subtype: AppPartitionSubType) -> Result<(), ()> {
        // This is a bit of cheating. We reach beyond what `CundaOta` struct suggests that it can do. It has no
        // reference to otadata partition but due to the limitations of esp_bootloader_esp_idf library we have to
        // cheat like this. Basically the ota helpers in the library requires accessing the entire flash storage.
        // That is why `SharedAsyncFlashRegion` actually allows reaching beyond the `region` it is defined with.
        self.new_app_slot
            .with_entire_flash_storage(
                |flash| -> Result<(), esp_bootloader_esp_idf::partitions::Error> {
                    let mut buffer = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
                    let pt = esp_bootloader_esp_idf::partitions::read_partition_table(
                        &mut *flash,
                        &mut buffer,
                    )
                    .expect("Error reading partition table");
                    let ota_part = pt
                        .find_partition(esp_bootloader_esp_idf::partitions::PartitionType::Data(
                            esp_bootloader_esp_idf::partitions::DataPartitionSubType::Ota,
                        ))?
                        .expect("Did not find OTA data partition");

                    let ota_part = ota_part.as_embedded_storage(flash);
                    let mut ota = ota::Ota::new(ota_part, 2)?;
                    ota.set_current_app_partition(subtype)?;
                    if subtype != AppPartitionSubType::Factory {
                        ota.set_current_ota_state(ota::OtaImageState::New)?;
                    }
                    Ok(())
                },
            )
            .await
            .map_err(|_err| ())
    }

    pub async fn approve_firmware(&mut self) -> Result<(), ()> {
        match &self.ota_state {
            OtaState::Ready => return Ok(()),
            OtaState::WritingNewFirmware(_firmware_verification_data) => return Err(()),
            OtaState::PendingVerify => {
                defmt::info!("Approving new firmware.");
                self.new_app_slot
                    .with_entire_flash_storage(|flash| {
                        let mut buffer = [0u8; partitions::PARTITION_TABLE_MAX_LEN];
                        let mut ota_updater = esp_bootloader_esp_idf::ota_updater::OtaUpdater::new(
                            flash,
                            &mut buffer,
                        )
                        .expect("Error reading the partition table");
                        if ota_updater
                            .set_current_ota_state(ota::OtaImageState::Valid)
                            .is_err()
                        {
                            return Err(());
                        } else {
                            self.ota_state = OtaState::Ready;
                            return Ok(());
                        }
                    })
                    .await
            }
        }
    }
}
