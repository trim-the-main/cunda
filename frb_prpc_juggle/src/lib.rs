#![cfg_attr(not(feature = "use_std"), no_std)]

#[cfg(feature = "use_std")]
pub mod client_interface;
mod macros;
