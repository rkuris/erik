# Next-Generation Firmware

This directory will host the rebuilt solar pool heater controller firmware with the following goals:

- Unified HTML interface served in both station and captive portal modes.
- Login-gated dashboard showing temperatures, relay state, and Wi-Fi status.
- Navigation for Wi-Fi provisioning, relay defaults, probe configuration, and admin tasks.
- Runtime storage of credentials and preferences in NVS.
- Hotspot provisioning fallback when no known network is reachable.

Implementation work will proceed here while the original code remains referenced in `docs/legacy-reference.md`.
