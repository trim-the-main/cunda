# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Cunda** is a BLE IoT project: an ESP32 firmware communicating with a Flutter mobile app over Bluetooth LE using an RPC protocol with binary (postcard/COBS) serialization.

```
firmware/          # ESP32 no_std Rust firmware (Embassy async runtime)
cunda_flutter/     # Flutter mobile app (BLE client + UI)
protocol/          # Shared Rust crate: RPC endpoints, topics, data types
frb_prpc_juggle/   # Bridge crate: Flutter-Rust Bridge ↔ postcard-rpc adapter
justfile           # Build automation
```

## Commands

### Firmware (ESP32 Rust)

```bash
just build-firmware-debug       # Build debug (DEFMT_LOG=info,firmware=debug)
just build-firmware-release     # Build release
just run-firmware-debug         # Flash + monitor debug build
just run-firmware-release       # Flash + monitor release build
```

Direct cargo commands (from `firmware/`):
```bash
DEFMT_LOG=info,firmware=debug cargo build
cargo run --release
```

### Flutter App

```bash
just codegen-flutter            # FRB codegen + build_runner (required after protocol changes)
just run-flutter                # codegen + run app
just run-flutter-fast           # Run without codegen (faster iteration)
```

Direct commands (from `cunda_flutter/`):
```bash
flutter_rust_bridge_codegen generate --stop-on-error
dart run build_runner build
RUST_LOG=debug flutter run --dart-define="LOG_LEVEL=Fine"
flutter test                    # Unit tests
flutter analyze                 # Lint
```

## Architecture

### Communication Flow

```
Flutter UI (Riverpod state)
    ↓ RPC call (generated client)
BLE GATT write → toServer characteristic (UUID ...0001)
    ↓ COBS-framed postcard bytes, MTU 255 (252 usable)
ESP32 postcard-rpc dispatcher
    ↓ deserialize → handler → serialize
BLE GATT Indication → toClient characteristic (UUID ...0002)
    ↓
Flutter client deserializes → provider update → UI re-render
```

BLE service UUID: `408813DF-5DD4-1F87-EC11-CDB001100000`

### Protocol Crate (`protocol/`)

The single source of truth for the RPC contract. Contains:
- **Endpoints**: RPC call/response pairs (callable by Flutter)
- **Topics**: Push streams from firmware to Flutter
- **Data types**: `SysStats`, `CpuUsage`, `MemoryUsage`, `Percent`, etc.

Uses a `flutter` feature flag: when enabled, uses `std::String` instead of `heapless::String`. The protocol types are shared — firmware uses the crate directly; Flutter uses generated Dart code from FRB codegen.

### Firmware (`firmware/src/`)

No-std Embassy async on dual-core ESP32:
- `main.rs`: Entry point, executor setup, task spawning
- `ble.rs`: trouble-host BLE GATT peripheral setup
- `rpc/`: postcard-rpc server dispatcher, BLE wire protocol, OTA handler
- `storage/`: Flash config storage (app_config, sys_config), OTA update manager
- `stats.rs`: CPU/memory/uptime tracking

Core 0 runs the Embassy async executor. Core 1 runs a busy-loop (simulates CPU load) — there's a known esp-hal bug (#4903) worked around in `main.rs`.

### Flutter App (`cunda_flutter/lib/`)

MVVM with Riverpod:
- `pages/scanner/`: BLE device discovery
- `pages/device/`: Main control UI (3 tabs: app settings, system stats, config)
  - `ota_section/`: OTA firmware update UI
- `services/ble/`: BLE abstraction layer (real + fake implementations)
- `providers/`: Riverpod providers for BLE adapter, scanner, RPC client

### Code Generation

Two codegen layers must both run when protocol changes:
1. **FRB codegen** (`flutter_rust_bridge_codegen generate`): generates Dart bindings from Rust types in `frb_prpc_juggle/`
2. **build_runner** (`dart run build_runner build`): generates `.g.dart` for Riverpod annotations and mockito mocks

## Key Toolchain Notes

- Firmware uses the **Espressif ESP Rust fork** (`rustup default esp`), not standard Rust
- Target: `xtensa-esp32-none-elf`
- OTA requires `bootloader_with_ota/bootloader.bin` and `partitions_two_ota_coredump.csv` (present in repo)
- Uses `espflash` runner (configured in `firmware/.cargo/config.toml`)
- GATT uses **Indications** (not Notifications) for reliable delivery
