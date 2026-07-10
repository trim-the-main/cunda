# tirbod: A navigation aid device for Tire Bouchon

Cunda Framework device with GPS receiver, alarm driver and N2K connectivity.

## Mission
- OneWire temperature sensors / alarms
- Keep track of GPS progress by showing a moving window average
- Keep track of Wind direction and speed, in also a moving window
- Calculate true wind speed using GPS, heading and Wind data

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
