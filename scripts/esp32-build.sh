#!/bin/bash
# ESP32-C6 Build Environment Setup
# DevOps-automated ESP-IDF configuration for Pool Controller firmware

set -e  # Exit on any error

echo "🔧 ESP32-C6 Build Environment Setup"
echo "====================================="

# Source ESP-IDF environment
if [ -f "/home/vscode/esp/esp-idf/export.sh" ]; then
    source /home/vscode/esp/esp-idf/export.sh
    echo "✅ ESP-IDF environment loaded"
else 
    echo "❌ ESP-IDF not found at expected location"
    exit 1
fi

# Export required environment variables for bindgen
export BINDGEN_EXTRA_CLANG_ARGS="-I/home/vscode/.espressif/tools/riscv32-esp-elf/esp-12.2.0_20230208/riscv32-esp-elf/lib/gcc/riscv32-esp-elf/12.2.0/include -I/home/vscode/.espressif/tools/riscv32-esp-elf/esp-12.2.0_20230208/riscv32-esp-elf/lib/gcc/riscv32-esp-elf/12.2.0/include-fixed"
export MCU=esp32c6

echo "✅ Bindgen environment configured for ESP32-C6"
echo "✅ Target MCU: $MCU"

# Run the requested command with the environment set up
if [ $# -eq 0 ]; then
    echo "Usage: $0 <cargo-command>"
    echo "Examples:"
    echo "  $0 check        # Check compilation"
    echo "  $0 build        # Build debug"
    echo "  $0 build --release  # Build release"
    echo "  $0 run --release    # Flash to device"
    exit 1
fi

echo "🚀 Running: cargo $@"
exec cargo "$@"