# Pool Controller Workspace

This repository now tracks both the original ESP32C6 firmware and the ongoing next-generation rewrite.

- `legacy/` – prior Rust/ESP-IDF project with JSON API and static Wi-Fi credentials.
- `nextgen/` – clean-slate implementation targeting an HTML UI, captive portal, and runtime provisioning.
- `docs/` – reference material, including `docs/legacy-reference.md` that documents the legacy behavior.

New development should occur under `nextgen/` while the `legacy/` directory remains available for reference and fallback builds.
