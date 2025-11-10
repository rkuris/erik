# DevContainer Optimized - Rebuild Required

## Recent Changes

The devcontainer has been **completely optimized** to eliminate slow startup times:

### What Changed

✅ **Pre-installed toolchains** - ESP-IDF, Rust, and cargo tools baked into the image  
✅ **Removed duplicate configurations** - Single clean Dockerfile and devcontainer.json  
✅ **Removed Playwright** - Not needed for firmware development  
✅ **Removed Agentic-Meetings clone** - Eliminated external dependency  
✅ **Added .dockerignore** - Faster build context (excludes target/, .git/, docs/)  
✅ **Minimal post-create** - Only installs web UI npm deps (fast)

### Performance Improvement

**Before:** 5-15 minutes (ESP-IDF clone + install + cargo tools + npm + Playwright)  
**After:** ~10-30 seconds (only npm install runs on startup)

The expensive operations (ESP-IDF installation, Rust toolchain, cargo-espflash) now run **once during image build** and are cached in Docker layers.

## Action Required

**Rebuild the devcontainer to apply these optimizations:**

1. Open the Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Select: **"Dev Containers: Rebuild Container"**
3. Wait for the **initial build** (~10-20 minutes one-time cost to build the image)
4. Future container starts will be **fast** (~10-30 seconds)

## What This Fixes

- **ESP-IDF Build Compatibility**: ESP-IDF v5.1.2 pre-installed for ESP32-C6
- **Consistent Workspace Path**: Mounted at `/workspaces/pool-controller` (no spaces)
- **Auto ESP Environment**: ESP-IDF environment variables sourced automatically
- **Node.js for Web UI**: Node.js 20.x LTS included for frontend builds
- **USB Device Access**: Configured for flashing firmware via USB

## After Rebuild

### Build Firmware
```bash
cd /workspaces/pool-controller/nextgen
cargo build --release
```

### Flash to Device
```bash
cd /workspaces/pool-controller/nextgen
cargo espflash flash --release --monitor
```

### Use VS Code Tasks
- **Build nextgen firmware** (Ctrl+Shift+B)
- **Check nextgen firmware**
- **Flash nextgen to device**
- **Clippy nextgen**
- **Format nextgen**

## Build Configuration

Optimized files:
- `.devcontainer/Dockerfile` - Single-stage image with all toolchains
- `.devcontainer/devcontainer.json` - Clean configuration, minimal post-create
- `.devcontainer/.dockerignore` - Excludes build artifacts from Docker context
- `nextgen/.cargo/config.toml` - Cargo defaults for ESP32-C6 target
- `.vscode/tasks.json` - Build and development tasks

## Installed Tools

Pre-installed in the container:
- ESP-IDF v5.1.2 (for ESP32-C6)
- Rust nightly with `rust-src`
- cargo-espflash, espflash, espmonitor, ldproxy
- Node.js 20.x LTS + npm
- GitHub CLI (gh)
- VS Code extensions: rust-analyzer, even-better-toml, crates, vscode-lldb
