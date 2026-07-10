# demo_esp32 firmware

ESP32 demo firmware written in `no_std` Rust using
[esp-hal](https://github.com/esp-rs/esp-hal), [Embassy](https://embassy.dev/)
async executor. Acts as the BLE peripheral and
[postcard-rpc](https://github.com/jamesmunns/postcard-rpc) RPC server.

Uses custom bootloader compiled with App rollback support.

## Features

- Simple GPIO functions as a demo app: Responds to a blink endpoint,
  publish topic messages for the button input
- Simple persistent firmware config using
  [sequential-storage](https::/github.com/tweedegolf/sequential-storage) key
  value store
- OTA update with App rollback
- Basic cpu/memory consumption and uptime stats published using topic messages
- Defmt logs over BLE to flutter, postcard topic as transport

## Build & flash

Requires the Espressif Rust toolchain to compile. Install using
[espup](https://github.com/esp-rs/espup).

```bash
# from repo root
just build-firmware-debug demo_esp32
just run-firmware-debug demo_esp32      # build + flash + monitor (debug)
just run-firmware-release demo_esp32    # build + flash + monitor (release)
```

Or directly from this directory:
```bash
DEFMT_LOG=info,firmware=debug cargo build
DEFMT_LOG=info,firmware=debug cargo run   # flashes via espflash runner
```

Log output is controlled by `DEFMT_LOG`. Set `firmware=debug` for verbose handler logs.
