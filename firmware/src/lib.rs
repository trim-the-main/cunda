#![no_std]
#![feature(macro_metavar_expr)]
#![feature(maybe_uninit_uninit_array_transpose)]
pub mod ble;
mod rpc;

mod stats;
pub mod storage;
