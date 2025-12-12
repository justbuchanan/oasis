#!/usr/bin/env bash
# Helper script to build ESP32 code on NixOS using the FHS environment
# This is needed because ESP-IDF toolchain binaries require an FHS environment on NixOS

set -e

cd "$(dirname "$0")"
OASIS_ROOT="$(cd ../.. && pwd)"

# Run cargo build in the FHS environment
exec "$OASIS_ROOT/result/bin/esp32-fhs" -c "unset PYTHONPATH; cargo run --bin oasis $*"
