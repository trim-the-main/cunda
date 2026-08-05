# Justfile for Cunda project

# default recipe lists the recipes
default:
    @echo "Cunda Project build recipes"
    @just --list

# Build and run the firmware (release)
# all: build-firmware-release run-firmware-release
# ==============================================================================
# Firmware
# ==============================================================================

_check-device device:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -d "device_firmwares/{{ device }}" ]; then
        echo "Error: device firmware does not exist for '{{ device }}'" >&2
        exit 1
    fi

# Build the firmware in debug mode
build-firmware-debug device: (_check-device device)
    #!/usr/bin/env bash
    set -euo pipefail
    (
      cd "device_firmwares/{{ device }}"
      DEFMT_LOG=info,postcard_rpc_ble=debug,{{ device }}=debug cargo build
    )

# Build the firmware in release mode
build-firmware-release device: (_check-device device)
    (cd device_firmwares/{{ device }} && cargo build --release)

# Run the firmware in debug mode (uses espflash runner from .cargo/config.toml)
run-firmware-debug device: (_check-device device)
    (cd device_firmwares/{{ device }} && DEFMT_LOG=info,postcard_rpc_ble=debug,{{ device }}=debug cargo run)

# Run the firmware in release mode
run-firmware-release device: (_check-device device)
    (cd device_firmwares/{{ device }} && cargo run --release)

# ==============================================================================
# Firmware Size Analysis
# ==============================================================================

# Path to ESP toolchain binutils
esp_tools := env('HOME') / ".rustup/toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin"

_check-elf-exists device type='release': (_check-device device)
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -f "device_firmwares/{{ device }}/target/xtensa-esp32-none-elf/{{ type }}/{{ device }}" ]; then
        echo "Error: device {{ type }} elf file does not exists for '{{ device }}'. Try building it." >&2
        exit 1
    fi

# Show section sizes (.text, .rodata, .data, .bss, etc.)
size-sections device type='release': (_check-elf-exists device type)
    {{ esp_tools }}/xtensa-esp32-elf-size -A device_firmwares/{{ device }}/target/xtensa-esp32-none-elf/{{ type }}/{{ device }}

# Show the N largest code symbols (default: 50)
[working-directory('device_firmwares')]
size-symbols device n='50' type='release': (_check-elf-exists device type)
    {{ esp_tools }}/xtensa-esp32-elf-nm --size-sort -S --radix=d {{ device }}/target/xtensa-esp32-none-elf/{{ type }}/{{ device }} | rustfilt | grep ' [tT] ' | tail -{{ n }}

# Aggregate code size by crate/module
size-crates device type='release': (_check-elf-exists device type)
    {{ esp_tools }}/xtensa-esp32-elf-nm --size-sort -S --radix=d device_firmwares/{{ device }}/target/xtensa-esp32-none-elf/{{ type }}/{{ device }} \
      | rustfilt \
      | grep ' [tT] ' \
      | awk '{size=$2; name=$4; split(name, parts, "::"); crate=parts[1]; sizes[crate]+=size} END {for (c in sizes) printf "%8d  %s\n", sizes[c], c}' \
      | sort -rn

# Source file attribution via bloaty (requires bloaty + debug info)
size-bloaty device n='50' type='release': (_check-elf-exists device type)
    bloaty device_firmwares/{{ device }}/target/xtensa-esp32-none-elf/{{ type }}/{{ device }} -d compileunits -n {{ n }}

# Differential size analysis between two ELF binaries
size-diff old new:
    bloaty {{ new }} -- {{ old }} -d compileunits

# Show Future/async state machine enum sizes, sorted by total size (requires nightly -Zprint-type-sizes) for release build
size-futures device filter='': (_check-elf-exists device "release")
    (   cd device_firmwares/{{ device }} \
        && RUSTFLAGS="-Zprint-type-sizes" cargo build --release 2>&1 \
           | grep 'print-type-size' \
           | grep -i '{{ filter }}' \
           | sort -t: -k2 -rn
    )

# Show Future/async state machine enum sizes, sorted by total size (requires nightly -Zprint-type-sizes) for debug build
size-futures-debug device filter='': (_check-elf-exists device "debug")
    (   cd device_firmwares/{{ device }} \
        && RUSTFLAGS="-Zprint-type-sizes" \
           DEFMT_LOG=info,postcard_rpc_ble=debug,{{ device }}=debug cargo build 2>&1 \
           | grep 'print-type-size' \
           | grep -i '{{ filter }}' \
           | sort -t: -k2 -rn
    )
# ==============================================================================
# Packaging
# ==============================================================================

# Build a .mayna distribution package from the release firmware ELF
package-firmware device: (build-firmware-release device)
    cargo run --manifest-path mayna/Cargo.toml -- create \
      --elf device_firmwares/{{ device }}/target/xtensa-esp32-none-elf/release/{{ device }} \
      --chip esp32 \
      --bootloader device_firmwares/{{ device }}/bootloader_with_ota/bootloader.bin \
      --output .

package-firmware-debug device: (build-firmware-debug device)
    cargo run --manifest-path mayna/Cargo.toml -- create \
      --elf device_firmwares/{{ device }}/target/xtensa-esp32-none-elf/debug/{{ device }} \
      --chip esp32 \
      --bootloader {{ device }}/bootloader_with_ota/bootloader.bin \
      --output .

# ==============================================================================
# Flutter
# ==============================================================================

# Run the Flutter application
[working-directory('cunda_flutter')]
run-flutter:
    flutter_rust_bridge_codegen generate --stop-on-error
    dart run build_runner build
    RUST_LOG=debug flutter run --dart-define="LOG_LEVEL=Fine"

# Run the Flutter application (fast, no codegen)
[working-directory('cunda_flutter')]
run-flutter-fast:
    RUST_LOG=debug flutter run --dart-define="LOG_LEVEL=Fine"

# Run the Flutter application with fake BLE (no hardware needed)
[working-directory('cunda_flutter')]
run-flutter-fake:
    RUST_LOG="debug" flutter run --dart-define="FAKE_BLE=true" --dart-define="LOG_LEVEL=Fine"

# Run the code generation only
[working-directory('cunda_flutter')]
codegen-flutter:
    flutter_rust_bridge_codegen generate --stop-on-error
    dart run build_runner build
