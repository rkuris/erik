# Pool Controller Workspace

ESP32C6 firmware for solar pool heater control with Wi-Fi provisioning and OTA updates.

- `legacy/` – Original Rust/ESP-IDF firmware with JSON API and static Wi-Fi credentials
- `nextgen/` – Active development: HTML UI, captive portal, runtime provisioning, OTA updates
- `OTA-PARTITION-LAYOUT.md` – Flash partition scheme and OTA update documentation

New development occurs under `nextgen/`. The `legacy/` directory remains for reference.

## Project Management

**GitHub Issues:** All work tracked as GitHub issues  
**Project Board:** https://github.com/users/Teonlight/projects/3  
**MVP Milestone:** https://github.com/rkuris/erik/milestone/1 (Due: 2025-11-10)

### Current Focus: Eliminate Physical Access

The MVP delivers remote firmware updates and Wi-Fi provisioning to eliminate physical device access:

- **Critical Path:** Issues #13, #14, #15 (OTA partition, upload, rollback)
- **High Priority:** Issues #8, #9, #10, #16 (Wi-Fi state machine, APIs, UI)
- **Post-MVP:** Security hardening (#2-7), testing infrastructure (#18-33)

See milestone for real-time progress.

## Development Personas

This workspace uses specialized AI personas (chatmodes) to maintain clear domain boundaries and accountability:

| Persona | Scope | GitHub Label |
|---------|-------|--------------|
| **Backend Developer** | Firmware implementation (`nextgen/src`, `legacy/src`) | `persona:backend` |
| **Frontend Developer** | Web UI assets (`nextgen/webui`) | `persona:frontend` |
| **Build Systems Engineer** | Compiler flags, size optimizations, CI/CD, OTA packaging | `persona:build-systems` |
| **Platform Engineer** | Partition layout, memory allocation, flash/RAM constraints | `persona:platform` |
| **QA Test Engineer** | Test planning, execution, coverage tracking | `persona:qa` |
| **Web Security Expert** | Security reviews and remediation guidance | `persona:security` |
| **Project Manager** | Documentation, status tracking, task coordination | `persona:pm` |

**Cross-Reference Protocol:** Build Systems Engineer and Platform Engineer coordinate on size/partition changes affecting both compiler output and flash layout. All personas delegate work outside their scope using the protocol in `.github/copilot-instructions.md`.

