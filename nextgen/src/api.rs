#[derive(Serialize)]
pub struct FirmwareDescriptor {
    pub name: &'static str,
    pub version: &'static str,
    pub build_time: &'static str,
    pub idf_version: &'static str,
}

impl Default for FirmwareDescriptor {
    fn default() -> Self {
        Self {
            name: "pool-controller",
            version: "unknown",
            build_time: "unknown",
            idf_version: "unknown",
        }
    }
}
use serde::{Deserialize, Serialize};

use crate::wifi::{WifiMode, WifiSnapshot};

#[derive(Clone, Serialize)]
pub struct StatusResponse {
    pub wifi: WifiStatus,
    pub relay: RelayStatus,
    pub probes: Vec<ProbeInfo>,
    #[serde(rename = "uptimeSeconds")]
    pub uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<FirmwareInfo>,
    #[serde(rename = "provisioningState")]
    pub provisioning_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ap: Option<ApInfo>,
}

#[derive(Serialize)]
pub struct ProvisioningStatusResponse {
    pub provisioned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct WifiStatus {
    pub mode: String,
    pub ssid: String,
    pub connected: bool,
    pub rssi: Option<i32>,
    pub ip: Option<String>,
}

impl WifiStatus {
    pub fn from_snapshot(snapshot: &WifiSnapshot) -> Self {
        let (mode, ssid) = match snapshot.mode {
            WifiMode::Station => (
                "STA".to_string(),
                snapshot.station.ssid.clone().unwrap_or_default(),
            ),
            WifiMode::AccessPoint => (
                "AP".to_string(),
                snapshot
                    .access_point
                    .as_ref()
                    .map_or_else(|| "Solar-Heater".to_string(), |ap| ap.ssid.clone()),
            ),
        };

        Self {
            mode,
            ssid,
            connected: snapshot.station.connected,
            rssi: snapshot.station.rssi,
            ip: snapshot.station.ip.clone(),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct RelayStatus {
    pub state: String,
    #[serde(rename = "lastChange")]
    pub last_change: Option<String>,
}

impl Default for RelayStatus {
    fn default() -> Self {
        Self {
            state: "off".into(),
            last_change: None,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct ProbeInfo {
    pub id: String,
    pub name: Option<String>,
    pub fahrenheit: Option<f32>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Serialize)]
pub struct Defaults {
    pub default_state: String,
    pub hysteresis: u16,
    #[serde(rename = "min_on_temp")]
    pub min_on_temp: u16,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            default_state: "off".into(),
            hysteresis: 2,
            min_on_temp: 70,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct ApInfo {
    pub ssid: String,
    pub channel: u8,
    #[serde(rename = "clientCount")]
    pub client_count: u16,
}

#[derive(Clone, Serialize)]
pub struct FirmwareInfo {
    pub sha256: String,
    pub size: u64,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: Option<String>,
    pub staged: bool,
    #[serde(rename = "currentSlot", skip_serializing_if = "Option::is_none")]
    pub current_slot: Option<String>,
    #[serde(rename = "pendingSlot", skip_serializing_if = "Option::is_none")]
    pub pending_slot: Option<String>,
    #[serde(rename = "previousSlot", skip_serializing_if = "Option::is_none")]
    pub previous_slot: Option<String>,
    #[serde(rename = "slotState", skip_serializing_if = "Option::is_none")]
    pub slot_state: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RelayRequest {
    pub state: String,
}

#[derive(Deserialize)]
pub struct DefaultsRequest {
    pub default_state: String,
    pub hysteresis: u16,
    pub min_on_temp: u16,
}

#[derive(Deserialize)]
pub struct WifiSaveRequest {
    pub ssid: String,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct PasswordChangeRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct ProvisioningRequest {
    pub username: Option<String>,
    pub password: String,
}
