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
    DEFMT_LOG=debug cargo build

# Build the firmware in release mode
[working-directory('firmware')]
build-firmware-release:
    cargo build --release

# Run the firmware in debug mode (uses espflash runner from .cargo/config.toml)
[working-directory('firmware')]
run-firmware-debug:
    DEFMT_LOG=debug cargo run

# Run the firmware in release mode
[working-directory('firmware')]
run-firmware-release:
    cargo run --release

# ==============================================================================
# Flutter
# ==============================================================================

# Run the Flutter application
[working-directory('cunda_flutter')]
run-flutter:
    flutter_rust_bridge_codegen generate --stop-on-error
    dart run build_runner build
    flutter run
