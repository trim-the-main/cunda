#![no_std]
#![feature(macro_metavar_expr)]
#![feature(maybe_uninit_uninit_array_transpose)]

extern crate alloc;

pub mod ble;
mod rpc;

mod stats;
pub mod storage;

pub mod i2c;

// pub mod oled_screen;
// pub mod screen_service;

pub mod temperature_service;

pub mod ublox_nmea_service; // Read nmea sentences from ublox device, publish them on the nmea channel

pub mod nmea_gps_parser;

pub mod context;
pub mod diagnostic_helpers;

defmt::timestamp!("{=u64} us", embassy_time::Instant::now().as_micros());
