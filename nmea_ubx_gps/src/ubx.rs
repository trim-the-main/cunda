extern crate alloc;

use alloc::vec::Vec;

use ublox::cfg_tx_ready::TxReadyIFace;
use ublox::cfg_val::CfgVal::*;
use ublox::packets::cfg_val::{CfgLayerSet, CfgValSetBuilder};

pub fn config_sam_m10q_i2c(buf: &mut Vec<u8>) {
    CfgValSetBuilder {
        version: 1,
        layers: CfgLayerSet::RAM,
        reserved1: 0,
        cfg_data: &[
            Uart1Enabled(false),
            I2cEnabled(true),
            I2cOutProtUbx(true),
            I2cOutProtNmea(true),
            RateMeas(1000),
            RateNav(1),
            MsgOutNmeaIdGgaI2c(1),
            MsgOutNmeaIdRmcI2c(1),
            MsgOutNmeaIdGllI2c(0),
            MsgOutNmeaIdGsaI2c(0),
            MsgOutNmeaIdGsvI2c(0),
            MsgOutNmeaIdVtgI2c(0),
            MsgOutUbxNavPvtI2c(1),
            TxReadyEnabled(true),
            TxReadyInterface(TxReadyIFace::I2C),
            TxReadyPin(4),
            TxReadyPolarity(false), // high active
            TxReadyThreshold(1),    // 8 bytes
        ],
    }
    .extend_to(buf);
}

pub fn new_parser() -> ublox::Parser<Vec<u8>, ublox::proto33::Proto33> {
    ublox::Parser::default()
}
