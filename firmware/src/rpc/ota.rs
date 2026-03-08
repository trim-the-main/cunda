
use postcard_rpc::header::VarHeader;
use protocol::endpoints::{EmptyRes, NoArg, OtaBytes, OtaMData, OtaState};

use crate::rpc::dispatcher::DispatchContext;

pub(super) async fn prepare(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: OtaMData,
) -> OtaState {
    defmt::debug!("Handling PrepareOta");
    OtaState::TransferReady(0)
}
pub(super) async fn transfer_bytes(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: OtaBytes,
) -> OtaState {
    defmt::debug!("Handling TransferOtaBytes");
    OtaState::TransferReady(0)
}
pub(super) async fn approve_firmware(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> EmptyRes {
    defmt::debug!("Handling ApproveFirmware");
    EmptyRes {}
}
pub(super) async fn finalize(
    _context: &mut DispatchContext,
    _header: VarHeader,
    _rqst: NoArg,
) -> OtaState {
    defmt::debug!("Handling FinalizeOta");
    OtaState::TransferReady(0)
}
