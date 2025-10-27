use std::sync::Mutex;

use anyhow::Result;
use log::{error, warn};
use once_cell::sync::{Lazy, OnceCell};
use serde::Serialize;

/// High-level provisioning states reported to the UI.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProvisioningState {
    Idle,
    Connecting,
    ApMode,
    Error,
}

impl Default for ProvisioningState {
    fn default() -> Self {
        ProvisioningState::ApMode
    }
}

/// Snapshot of the station-specific status.
#[derive(Clone, Debug, Default, Serialize)]
pub struct StationSnapshot {
    pub ssid: Option<String>,
    pub connected: bool,
    pub rssi: Option<i32>,
    pub ip: Option<String>,
}

/// Snapshot of the access-point status.
#[derive(Clone, Debug, Serialize)]
pub struct AccessPointSnapshot {
    pub ssid: String,
    pub channel: u8,
    #[serde(rename = "clientCount")]
    pub client_count: u16,
}

impl Default for AccessPointSnapshot {
    fn default() -> Self {
        AccessPointSnapshot {
            ssid: "Solar-Heater".into(),
            channel: 1,
            client_count: 0,
        }
    }
}

/// Aggregated Wi-Fi state exposed to the rest of the firmware.
#[derive(Clone, Debug, Serialize)]
pub struct WifiSnapshot {
    #[serde(rename = "mode")]
    pub mode: WifiMode,
    #[serde(rename = "station")]
    pub station: StationSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_point: Option<AccessPointSnapshot>,
    #[serde(rename = "provisioningState")]
    pub provisioning_state: ProvisioningState,
}

impl Default for WifiSnapshot {
    fn default() -> Self {
        WifiSnapshot {
            mode: WifiMode::AccessPoint,
            station: StationSnapshot::default(),
            access_point: Some(AccessPointSnapshot::default()),
            provisioning_state: ProvisioningState::ApMode,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum WifiMode {
    Station,
    AccessPoint,
}

impl Default for WifiMode {
    fn default() -> Self {
        WifiMode::AccessPoint
    }
}

/// Representation of a scanned network that we can serialize directly.
#[derive(Clone, Debug, Serialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub rssi: i32,
    pub secure: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct WifiScanResponse {
    pub networks: Vec<WifiNetwork>,
}

#[derive(Default)]
struct WifiRuntime {
    snapshot: WifiSnapshot,
    scan_cache: Vec<WifiNetwork>,
}

impl WifiRuntime {
    fn new() -> Self {
        Self {
            snapshot: WifiSnapshot::default(),
            scan_cache: vec![
                WifiNetwork {
                    ssid: "Backyard".into(),
                    rssi: -55,
                    secure: true,
                },
                WifiNetwork {
                    ssid: "Guest".into(),
                    rssi: -68,
                    secure: false,
                },
            ],
        }
    }
}

/// Centralized Wi-Fi state owner. Hardware integrations will update this
/// structure, while HTTP handlers consume snapshots.
pub struct WifiController {
    inner: Mutex<WifiRuntime>,
}

impl WifiController {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(WifiRuntime::new()),
        }
    }

    /// Returns a copy of the current status.
    pub fn snapshot(&self) -> WifiSnapshot {
        self.inner.lock().unwrap().snapshot.clone()
    }

    /// Marks the beginning of a STA join attempt for the provided SSID.
    pub fn begin_sta_attempt(&self, ssid: &str) {
        let mut guard = self.inner.lock().unwrap();
        guard.snapshot.mode = WifiMode::Station;
        guard.snapshot.provisioning_state = ProvisioningState::Connecting;
        guard.snapshot.station.connected = false;
        guard.snapshot.station.ssid = Some(ssid.to_owned());
        guard.snapshot.station.rssi = None;
        guard.snapshot.station.ip = None;
        guard.snapshot.access_point = None;
    }

    /// Records a successful STA join.
    pub fn mark_sta_connected(&self, ssid: &str, rssi: Option<i32>, ip: Option<String>) {
        let mut guard = self.inner.lock().unwrap();
        guard.snapshot.mode = WifiMode::Station;
        guard.snapshot.provisioning_state = ProvisioningState::Idle;
        guard.snapshot.station.ssid = Some(ssid.to_owned());
        guard.snapshot.station.connected = true;
        guard.snapshot.station.rssi = rssi;
        guard.snapshot.station.ip = ip;
    }

    /// Fallback to AP mode, typically after repeated STA failures.
    pub fn enable_captive_ap(&self, ssid: Option<String>) {
        let mut guard = self.inner.lock().unwrap();
        guard.snapshot.mode = WifiMode::AccessPoint;
        guard.snapshot.provisioning_state = ProvisioningState::ApMode;
        guard.snapshot.station = StationSnapshot::default();
        let mut ap = guard
            .snapshot
            .access_point
            .take()
            .unwrap_or_else(AccessPointSnapshot::default);
        if let Some(custom_ssid) = ssid {
            ap.ssid = custom_ssid;
        }
        guard.snapshot.access_point = Some(ap);
    }

    /// Marks the provisioning state as failed and returns to AP mode.
    pub fn mark_error(&self) {
        let mut guard = self.inner.lock().unwrap();
        guard.snapshot.provisioning_state = ProvisioningState::Error;
        guard.snapshot.station.connected = false;
    }

    /// Updates the most recent scan results (currently stubbed with cached data).
    pub fn scan_networks(&self) -> WifiScanResponse {
        #[cfg(target_os = "espidf")]
        {
            match hardware::scan_networks() {
                Ok(networks) => {
                    let mut guard = self.inner.lock().unwrap();
                    guard.scan_cache = networks.clone();
                    return WifiScanResponse { networks };
                }
                Err(err) => warn!("Hardware Wi-Fi scan failed: {err}"),
            }
        }

        let networks = self.inner.lock().unwrap().scan_cache.clone();
        WifiScanResponse { networks }
    }
}

pub static CONTROLLER: Lazy<WifiController> = Lazy::new(WifiController::new);

#[cfg(target_os = "espidf")]
mod hardware {
    use super::{WifiNetwork, CONTROLLER};
    use anyhow::{anyhow, Context, Result};
    use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
    use esp_idf_svc::{
        eventloop::EspSystemEventLoop,
        hal::prelude::Peripherals,
        wifi::{BlockingWifi, EspWifi},
    };
    use heapless::String as HeaplessString;
    use once_cell::sync::OnceCell;
    use std::{sync::Mutex, thread};

    use crate::NVS_PARTITION;

    struct Driver {
        wifi: Mutex<BlockingWifi<EspWifi<'static>>>,
        _sysloop: EspSystemEventLoop,
    }

    static DRIVER: OnceCell<Driver> = OnceCell::new();

    pub(super) fn initialize() -> Result<()> {
        DRIVER.get_or_try_init(|| {
            let peripherals =
                Peripherals::take().ok_or_else(|| anyhow!("ESP peripherals already taken"))?;
            let sysloop = EspSystemEventLoop::take()?;
            let nvs = NVS_PARTITION.clone();
            let wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?;
            let blocking = BlockingWifi::wrap(wifi, sysloop.clone())?;
            Ok(Driver {
                wifi: Mutex::new(blocking),
                _sysloop: sysloop,
            })
        })?;
        Ok(())
    }

    pub(super) fn schedule_sta_connect(ssid: String, password: Option<String>) -> Result<()> {
        initialize()?;
        thread::spawn(move || {
            if let Err(err) = connect_sta(ssid.clone(), password) {
                error!("STA connection attempt failed for '{ssid}': {err}");
                CONTROLLER.mark_error();
            }
        });
        Ok(())
    }

    pub(super) fn scan_networks() -> Result<Vec<WifiNetwork>> {
        initialize()?;
        let driver = DRIVER
            .get()
            .ok_or_else(|| anyhow!("Wi-Fi driver not initialized"))?;
        let mut wifi = driver.wifi.lock().unwrap();

        if !matches!(wifi.is_started(), Ok(true)) {
            wifi.start().context("Failed to start Wi-Fi before scan")?;
        }

        let results = wifi.scan().context("Wi-Fi scan failed")?;
        let networks = results
            .into_iter()
            .map(|ap| WifiNetwork {
                ssid: ap.ssid.as_str().to_owned(),
                rssi: ap.rssi,
                secure: ap.auth_method != AuthMethod::None,
            })
            .collect();
        Ok(networks)
    }

    fn connect_sta(ssid: String, password: Option<String>) -> Result<()> {
        initialize()?;
        let driver = DRIVER
            .get()
            .ok_or_else(|| anyhow!("Wi-Fi driver not initialized"))?;
        let mut wifi = driver.wifi.lock().unwrap();

        if matches!(wifi.is_started(), Ok(true)) {
            let _ = wifi.disconnect();
            let _ = wifi.stop();
        }

        let client_config = build_client_config(&ssid, password.as_deref())?;
        wifi.set_configuration(&Configuration::Client(client_config))?;
        wifi.start()?;
        wifi.connect()?;
        wifi.wait_netif_up()
            .context("Interface failed to obtain network parameters")?;

        let ip_address = match wifi.wifi().sta_netif().get_ip_info() {
            Ok(info) => Some(info.ip.to_string()),
            Err(err) => {
                warn!("Failed to read station IP info: {err}");
                None
            }
        };

        let rssi = match wifi.wifi_mut().driver_mut().get_ap_info() {
            Ok(info) => Some(info.rssi),
            Err(err) => {
                warn!("Failed to read AP info: {err}");
                None
            }
        };

        drop(wifi);

        CONTROLLER.mark_sta_connected(&ssid, rssi, ip_address);

        Ok(())
    }

    fn build_client_config(ssid: &str, password: Option<&str>) -> Result<ClientConfiguration> {
        let ssid_value = to_heapless::<32>(ssid)?;
        let mut password_value: HeaplessString<64> = HeaplessString::new();
        if let Some(secret) = password {
            password_value
                .push_str(secret)
                .map_err(|_| anyhow!("Wi-Fi password exceeds 64 characters"))?;
        }

        let auth_method = if password_value.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        };

        Ok(ClientConfiguration {
            ssid: ssid_value,
            bssid: None,
            auth_method,
            password: password_value,
            ..Default::default()
        })
    }

    fn to_heapless<const N: usize>(value: &str) -> Result<HeaplessString<N>> {
        let mut result = HeaplessString::<N>::new();
        result
            .push_str(value)
            .map_err(|_| anyhow!("Value exceeds {} characters", N))?;
        Ok(result)
    }
}

#[cfg(not(target_os = "espidf"))]
mod hardware {
    use super::WifiNetwork;
    use anyhow::{anyhow, Result};

    pub(super) fn initialize() -> Result<()> {
        Ok(())
    }

    pub(super) fn schedule_sta_connect(_ssid: String, _password: Option<String>) -> Result<()> {
        Ok(())
    }

    pub(super) fn scan_networks() -> Result<Vec<WifiNetwork>> {
        Err(anyhow!("Wi-Fi scanning not available on this target"))
    }
}

pub fn initialize() -> Result<()> {
    hardware::initialize()
}

pub fn schedule_sta_connect(ssid: String, password: Option<String>) -> Result<()> {
    hardware::schedule_sta_connect(ssid, password)
}
