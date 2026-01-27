#![cfg_attr(not(feature = "use_std"), no_std)]
pub mod accumulator;

#[cfg(feature = "use_std")]
pub mod client_interface;
mod macros;
