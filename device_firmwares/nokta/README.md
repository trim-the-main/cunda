# nokta: GPS + Anchor alarm

Cunda Framework device with GPS receiver. It has an NMEA 0183 output for GPS
messages. It implements a basic GPS functionality and an anchor alarm.

## Build & flash

Requires the Espressif Rust toolchain to compile. Install using
[espup](https://github.com/esp-rs/espup).

```bash
# from repo root
just build-firmware-debug nokta
just run-firmware-debug nokta      # build + flash + monitor (debug)
just run-firmware-release nokta     # build + flash + monitor (release)
```

Or directly from this directory:
```bash
DEFMT_LOG=info,firmware=debug cargo build
DEFMT_LOG=info,firmware=debug cargo run   # flashes via espflash runner
```

Log output is controlled by `DEFMT_LOG`. Set `firmware=debug` for verbose handler logs.
