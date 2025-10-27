# Legacy Reference

This document captures the behavior of the pre-redesign firmware so the original workflow remains easy to consult while the new application is built in parallel.

## High-Level Behavior

- Firmware entry point: `src/main.rs`.
- Platform: Seeed Studio XIAO ESP32C6 using the `esp-idf-hal` crate.
- Services: blocking Wi-Fi station connection, DS18B20 temperature polling, relay GPIO control, minimal HTTP server that returns JSON.

## Wi-Fi Lifecycle

- Reads a static list of SSID/password pairs from `src/secrets.rs` (generated from `src/secrets.rs.example`).
- On boot, scans for nearby APs and connects to the first SSID that matches the static list.
- Retries in a loop on failure; no captive portal, credential entry, or dynamic provisioning.

## HTTP Interface

- Single handler on `/` responding to `GET`.
- Accepts optional query parameter `relay=on|off` to toggle the heater relay state.
- Responds with handcrafted JSON containing:
  - `units`: temperature unit (Fahrenheit).
  - `relay`: current relay state (boolean).
  - `sensors`: JSON object keyed by OneWire device addresses with latest Fahrenheit readings, plus the on-chip internal temperature sensor.

## Sensors and Hardware Mappings

- Internal ESP32C6 temperature sensor via `TempSensorDriver`.
- Two external DS18B20 sensors on GPIO2 and GPIO21 (individual OneWire buses with pull-up resistors).
- Relay output on GPIO1 (active high).
- Status LED on GPIO15, toggled every two seconds in the main loop.

## Persistence and Preferences

- Uses ESP32 NVS namespace `pool` to load hysteresis (`u16`) and minimum-on temperature (`u8`) defaults (currently fixed defaults of 2°F and 70°F, respectively).
- Relay state cached in a global `OnceLock<Mutex<bool>>` to survive within-session changes; no NVS persistence for relay or Wi-Fi credentials.

## Known Limitations (Motivating the Redesign)

- Static Wi-Fi credential list requires recompiling firmware for new networks.
- Query-parameter control surface is insecure and difficult to use from browsers.
- No authentication, HTTPS, or user interface beyond raw JSON.
- No captive portal or AP fallback for provisioning.
- No structured configuration of probes, defaults, or admin operations via the web interface.

This snapshot should provide enough historical context when referencing the original workflow during the rebuild.
