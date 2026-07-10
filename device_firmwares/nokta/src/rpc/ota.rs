use postcard_rpc::header::VarHeader;
use protocol::cunda_common::v1::types::{NoArg, OtaBytes, OtaMData, OtaResult};

use crate::rpc::context::DispatchContext;

pub(super) async fn prepare(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: OtaMData,
) -> OtaResult {
    defmt::debug!("Handling PrepareOta");
    let mut ota = crate::storage::ota::OTA.lock().await;
    if let Some(ref mut ota) = *ota {
        if ota.prepare(rqst.size, rqst.hash_sha256).await.is_err() {
            OtaResult::StorageError
        } else {
            OtaResult::TransferReady
        }
    } else {
        OtaResult::NotSupported
    }
}

pub(super) async fn transfer_bytes(
    _context: &mut DispatchContext,
    _header: VarHeader,
    rqst: OtaBytes,
) -> OtaResult {
    defmt::debug!("Handling TransferOtaBytes");
    let mut ota = crate::storage::ota::OTA.lock().await;
    if let Some(ref mut ota) = *ota {
        match ota.write_bytes(rqst.offset, &rqst.data).await {
            Ok(_) => OtaResult::TransferReady,
            Err(e) => {
                defmt::error!("FlashStorageError during ota transfer_bytes: {}", e);
                OtaResult::StorageError
            }
        }
    } else {
        OtaResult::NotSupported
    }
}

pub(super) async fn finalize(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> OtaResult {
    defmt::debug!("Handling FinalizeOta");
    let mut ota = crate::storage::ota::OTA.lock().await;
    if let Some(ref mut ota) = *ota {
        let mut buffer = [0u8; 256];
        if ota
            .verify_new_app_transferred_correctly(&mut buffer)
            .await
            .is_err()
        {
            defmt::error!("Verification of new firmware bytes failed");
            return OtaResult::VerificationError;
        }
        match ota.mark_new_firmware_for_boot().await {
            Ok(_) => {
                defmt::warn!("Restarting after OTA update");
                esp_hal::system::software_reset()
            }
            Err(_) => {
                defmt::error!("Failed to mark the new partition for boot");
                return OtaResult::StorageError;
            }
        }
    } else {
        OtaResult::NotSupported
    }
}

pub(super) async fn approve_firmware(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> OtaResult {
    defmt::debug!("Handling ApproveFirmware");
    let _timer = crate::LogTimeOfScope::new("Handling of ApproveFirmware");

    let mut ota = crate::storage::ota::OTA.lock().await;
    if let Some(ref mut ota) = *ota {
        match ota.approve_firmware().await {
            Ok(_) => OtaResult::TransferReady,
            Err(_) => OtaResult::StorageError,
        }
    } else {
        OtaResult::NotSupported
    }
}

pub(super) async fn factory_reset(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> OtaResult {
    defmt::debug!("Handling FactoryReset");
    let mut ota = crate::storage::ota::OTA.lock().await;
    if let Some(ref mut ota) = *ota {
        match ota.mark_factory_for_boot().await {
            Ok(_) => {
                defmt::warn!("Restarting, booting factory app");
                esp_hal::system::software_reset()
            }
            Err(err) => {
                defmt::error!("Error during factory reset {}", err);
                OtaResult::StorageError
            }
        }
    } else {
        OtaResult::NotSupported
    }
}
