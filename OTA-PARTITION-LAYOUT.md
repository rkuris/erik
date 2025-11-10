# OTA Partition Layout

**Status:** ✅ Implemented (Issue #13)  
**Location:** `nextgen/partitions.csv`  
**Last Updated:** 2025-11-09

## Overview

The ESP32C6 firmware uses a custom partition table to support Over-The-Air (OTA) updates with rollback capability. The partition scheme includes factory firmware, dual OTA slots, and persistent storage for Wi-Fi credentials and firmware metadata.

## Partition Table

```csv
# Name,   Type, SubType, Offset,  Size,   Flags
nvs,      data, nvs,     0x9000,  0x6000,
phy_init, data, phy,     0xf000,  0x1000,
factory,  app,  factory, 0x10000, 1M,
ota_0,    app,  ota_0,   ,        1M,
ota_1,    app,  ota_1,   ,        1M,
```

## Partition Details

### NVS (Non-Volatile Storage)
- **Offset:** `0x9000`
- **Size:** `0x6000` (24 KB)
- **Purpose:** Stores Wi-Fi credentials, firmware metadata, and application configuration
- **Usage:** 
  - Wi-Fi SSID and passphrase persistence
  - OTA metadata (firmware version, SHA256, boot counts)
  - Application settings (defaults, probe configuration)

### PHY Initialization Data
- **Offset:** `0xf000`
- **Size:** `0x1000` (4 KB)
- **Purpose:** ESP32C6 radio calibration and PHY initialization parameters

### Factory Application
- **Offset:** `0x10000` (64 KB)
- **Size:** `1M` (1 MB)
- **Purpose:** Initial/recovery firmware image
- **Usage:**
  - Pre-flashed during manufacturing or development
  - Fallback target if both OTA slots are corrupted
  - Not overwritten by OTA updates

### OTA Slot 0
- **Offset:** Auto-assigned (follows factory partition)
- **Size:** `1M` (1 MB)
- **Purpose:** First OTA update slot (A/B partition scheme)

### OTA Slot 1
- **Offset:** Auto-assigned (follows OTA_0)
- **Size:** `1M` (1 MB)
- **Purpose:** Second OTA update slot (A/B partition scheme)

## OTA Boot Flow

1. **Bootloader** reads OTA metadata from NVS
2. **Active slot determination:** Bootloader selects `ota_0` or `ota_1` based on metadata
3. **Boot from active slot:** Application starts from selected partition
4. **Health check:** Application validates boot success (see #15 for rollback mechanism)
5. **Mark boot successful:** If healthy, application confirms slot as valid

## Rollback Mechanism

(See Issue #15 - OTA Rollback Mechanism for implementation details)

When implemented, the rollback flow will:
- Mark new OTA images as "pending" until health checks pass
- Track boot counts to detect boot loops
- Automatically roll back to previous slot if validation fails
- Expose rollback state via `/api/status` for UI visibility

## Flash Constraints

### Size Budget
- **Per-slot capacity:** 1 MB
- **Current firmware size (release):** ~600-800 KB (varies with features)
- **Headroom:** ~200-400 KB per slot for future feature growth

### Flash Layout
Total flash layout (3.1 MB used):
```
0x0000   - Bootloader (ESP-IDF managed)
0x9000   - NVS (24 KB)
0xf000   - PHY Init (4 KB)
0x10000  - Factory (1 MB)
~0x110000 - OTA_0 (1 MB)
~0x210000 - OTA_1 (1 MB)
```

## Build Integration

The partition table is automatically integrated into the build process via `build.rs`:

```rust
// nextgen/build.rs copies partitions.csv to CARGO_OUT_DIR
// ESP-IDF build picks it up via CONFIG_PARTITION_TABLE_CUSTOM_FILENAME
```

Configuration in `sdkconfig.defaults`:
```
CONFIG_PARTITION_TABLE_CUSTOM=y
CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"
```

## Flashing Instructions

### Development (via espflash)
```bash
cd nextgen
cargo espflash flash --release --monitor
```

The partition table is flashed automatically with the firmware.

### Production (via esptool.py)
```bash
# Flash partition table
esptool.py --chip esp32c6 write_flash 0x8000 partitions.bin

# Flash factory firmware
esptool.py --chip esp32c6 write_flash 0x10000 firmware.bin
```

## Verification

After flashing, verify partition table:

```bash
# Connect to device serial monitor
cargo espflash monitor

# Look for bootloader logs showing partition table:
# I (xxx) boot: Partition Table:
# I (xxx) boot: ## Label            Usage          Type ST Offset   Length
# I (xxx) boot:  0 nvs              WiFi data        01 02 00009000 00006000
# I (xxx) boot:  1 phy_init         RF data          01 01 0000f000 00001000
# I (xxx) boot:  2 factory          factory app      00 00 00010000 00100000
# I (xxx) boot:  3 ota_0            OTA app          00 10 00110000 00100000
# I (xxx) boot:  4 ota_1            OTA app          00 11 00210000 00100000
```

## Known Limitations

- **No data partition:** Application state must fit in NVS or be managed externally
- **Fixed slot sizes:** Cannot dynamically resize OTA partitions without reflashing
- **No encryption:** Partition table and flash contents are not encrypted (post-MVP security hardening)

## Related Issues

- **#13** - OTA Partition Scheme (implementation - COMPLETE)
- **#14** - Secure Firmware Upload (uses OTA slots)
- **#15** - OTA Rollback Mechanism (validates boot from OTA slots)
- **#16** - OTA Update UI (displays slot status and firmware metadata)

## Future Enhancements

- Add encrypted flash partitions (post-MVP security)
- Add separate data partition for application state
- Consider larger OTA slots if feature growth exceeds 1MB budget
- Add partition for firmware signature storage (if not in NVS)

---

> Project-Manager
