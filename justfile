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

# Build the firmware in debug mode
[working-directory('firmware')]
build-firmware-debug:
    DEFMT_LOG=info,firmware=debug cargo build

# Build the firmware in release mode
[working-directory('firmware')]
build-firmware-release:
    cargo build --release

# Run the firmware in debug mode (uses espflash runner from .cargo/config.toml)
[working-directory('firmware')]
run-firmware-debug:
    DEFMT_LOG=info,firmware=debug,postcard_rpc_ble=debug cargo run

# Run the firmware in release mode
[working-directory('firmware')]
run-firmware-release:
    cargo run --release

# ==============================================================================
# Firmware Size Analysis
# ==============================================================================

# Path to ESP toolchain binutils
esp_tools := env('HOME') / ".rustup/toolchains/esp/xtensa-esp-elf/esp-15.2.0_20250920/xtensa-esp-elf/bin"
elf := "firmware/target/xtensa-esp32-none-elf/release/firmware"

# Show section sizes (.text, .rodata, .data, .bss, etc.)
size-sections:
    {{esp_tools}}/xtensa-esp32-elf-size -A {{elf}}

# Show the N largest code symbols (default: 50)
size-symbols n='50':
    {{esp_tools}}/xtensa-esp32-elf-nm --size-sort -S --radix=d {{elf}} | rustfilt | grep ' [tT] ' | tail -{{n}}

# Aggregate code size by crate/module
size-crates:
    {{esp_tools}}/xtensa-esp32-elf-nm --size-sort -S --radix=d {{elf}} \
      | rustfilt \
      | grep ' [tT] ' \
      | awk '{size=$2; name=$4; split(name, parts, "::"); crate=parts[1]; sizes[crate]+=size} END {for (c in sizes) printf "%8d  %s\n", sizes[c], c}' \
      | sort -rn

# Source file attribution via bloaty (requires bloaty + debug info)
size-bloaty n='50':
    bloaty {{elf}} -d compileunits -n {{n}}

# Differential size analysis between two ELF binaries
size-diff old new:
    bloaty {{new}} -- {{old}} -d compileunits

# Show Future/async state machine enum sizes, sorted by total size (requires nightly -Zprint-type-sizes)
[working-directory('firmware')]
size-futures filter='':
    RUSTFLAGS="-Zprint-type-sizes" cargo build --release 2>&1 \
      | grep 'print-type-size' \
      | grep -i '{{filter}}' \
      | sort -t: -k2 -rn

# ==============================================================================
# Packaging
# ==============================================================================

# Build a .mayna distribution package from the release firmware ELF
package-firmware: build-firmware-release
    cargo run --manifest-path mayna/Cargo.toml -- create \
      --elf {{elf}} \
      --chip esp32 \
      --bootloader firmware/bootloader_with_ota/bootloader.bin \
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

# Run the Flutter application
[working-directory('cunda_flutter')]
run-flutter-fast:
    RUST_LOG=debug flutter run --dart-define="LOG_LEVEL=Fine"

# Run the code generation only
[working-directory('cunda_flutter')]
codegen-flutter:
    flutter_rust_bridge_codegen generate --stop-on-error
    dart run build_runner build
