#![no_std]
#![feature(macro_metavar_expr)]
#![feature(maybe_uninit_uninit_array_transpose)]

use embassy_time::Instant;
pub mod ble;
mod rpc;

mod stats;
pub mod storage;

pub struct LogTimeOfScope {
    start: Instant,
    name: &'static str,
}

impl LogTimeOfScope {
    pub fn new(name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            name,
        }
    }
}

impl Drop for LogTimeOfScope {
    fn drop(&mut self) {
        let took = Instant::now() - self.start;
        defmt::info!("{} took {} ms", self.name, took.as_millis());
    }
}

defmt::timestamp!("{=u64} us", embassy_time::Instant::now().as_micros());
