// Demo device for cunda framework using esp32
// In addition to the common cunda functionality (Cpu/memory stats,
// system logs, sys settings and OTA) we add some toy functions:
//
// Implement simple blinker to illustrate endpoint calls,
// a simple button to illustrate topics to carry async events
// from the microcontroller and a bandwidth tester to show the
// capabilities, what's the data rate postcard-rpc over BLE can
// handle

pub const RPC_PROTOCOL_VERSION: u32 = 1;

pub mod endpoints;
pub mod topics;
pub mod types;
