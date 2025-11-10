//! Next-generation firmware scaffold for the solar pool heater controller.

mod api;
mod auth;
mod http;
mod ota;
mod storage;
mod util;
mod wifi;

use std::{thread, time::Duration};

use anyhow::Result;
use esp_idf_svc::{log::EspLogger, sys};
use log::{info, warn};

#[cfg(target_os = "espidf")]
pub use storage::NVS_PARTITION;

pub const SALT_LEN: usize = 16;
pub const TOKEN_LEN: usize = 32;
#[cfg(feature = "weak-derivation")]
pub const PBKDF2_ITERATIONS: u32 = 2_000; // Development override (feature gated)
#[cfg(not(feature = "weak-derivation"))]
pub const PBKDF2_ITERATIONS: u32 = 100_000; // Production strength
pub const MAX_FIRMWARE_SIZE: usize = 2 * 1024 * 1024;
pub const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

fn main() -> Result<()> {
    sys::link_patches();
    EspLogger::initialize_default();

    info!("Bootstrapping next-generation firmware scaffold");

    #[cfg(target_os = "espidf")]
    ota::handle_pending_ota_state();

    match wifi::initialize() {
    Ok(()) => attempt_sta_restore_from_nvs(),
        Err(err) => warn!("Wi-Fi initialization failed: {err:?}"),
    }

    let _server = http::start_http_server()?;
    info!("HTTP server started; serving embedded web UI");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn attempt_sta_restore_from_nvs() {
    match storage::load_wifi_credentials() {
        Ok(Some(credentials)) => {
            info!("Restoring Wi-Fi credentials for '{}'", credentials.ssid);
            wifi::CONTROLLER.begin_sta_attempt(&credentials.ssid);

            let storage::WifiCredentials { ssid, password } = credentials;

            if let Err(err) = wifi::schedule_sta_connect(ssid, password) {
                warn!("Failed to schedule STA reconnect: {err:?}");
                wifi::CONTROLLER.mark_error();
            }
        }
        Ok(None) => info!("No stored Wi-Fi credentials found; remaining in AP mode"),
        Err(err) => warn!("Failed to load Wi-Fi credentials from NVS: {err:?}"),
    }
}

