# firmware

ESP32 firmware written in `no_std` Rust using
[esp-hal](https://github.com/esp-rs/esp-hal), [Embassy](https://embassy.dev/)
async executor. Acts as the BLE peripheral and
[postcard-rpc](https://github.com/jamesmunns/postcard-rpc) RPC server.

Uses custom bootloader compiled with App rollback support.

## Features

- postcard_rpc endpoints
- publish outgoing topic messages (incoming topics not supported)
- Simple persistent firmware config using
  [sequential-storage](https::/github.com/tweedegolf/sequential-storage) key
  value store
- OTA update with App rollback
- Basic cpu/memory consumption and uptime stats published using topic messages

## Build & flash

Requires the Espressif Rust toolchain to compile. Install using
[espup](https://github.com/esp-rs/espup).

```bash
# from repo root
just build-firmware-debug
just run-firmware-debug       # build + flash + monitor (debug)
just run-firmware-release     # build + flash + monitor (release)
```

Or directly from this directory:
```bash
DEFMT_LOG=info,firmware=debug cargo build
DEFMT_LOG=info,firmware=debug cargo run   # flashes via espflash runner
```

Log output is controlled by `DEFMT_LOG`. Set `firmware=debug` for verbose handler logs.
