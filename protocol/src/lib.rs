#![cfg_attr(not(feature = "flutter"), no_std)]

#[macro_use]
pub mod type_helpers;

#[macro_use]
pub mod cunda_macros;

#[macro_use]
pub mod mayna_macros;

pub mod devices;
pub mod types;

#[cfg(feature = "flutter")]
pub mod cunda_defaults {
    pub use crate::types::cunda_defaults::*;
    pub use frb_prpc_juggle::client_interface::FrbPostcardRpcError;
}
