use std::{sync::Mutex, thread, time::Duration};

use esp_idf_svc::{
    http::server::{EspHttpConnection, Request},
    sys::{self, EspError},
};
use log::{error, info, warn};
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};

use crate::{
    api::FirmwareInfo,
    http::{self, AppState},
    util::{hex_encode, now_rfc3339},
    MAX_FIRMWARE_SIZE,
};

#[cfg(target_os = "espidf")]
use esp_idf_svc::ota::{EspOta, Slot, SlotState};
#[cfg(target_os = "espidf")]
use serde_json::json;

#[derive(Debug, PartialEq, Eq)]
pub enum FirmwareValidationError {
    Empty,
    TooLarge(usize),
}

impl FirmwareValidationError {
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Empty => "Firmware image cannot be empty",
            Self::TooLarge(_) => "Firmware image exceeds 2 MiB limit",
        }
    }
}

pub const fn enforce_firmware_constraints(len: usize) -> Result<(), FirmwareValidationError> {
    if len == 0 {
        return Err(FirmwareValidationError::Empty);
    }

    if len > MAX_FIRMWARE_SIZE {
        return Err(FirmwareValidationError::TooLarge(len));
    }

    Ok(())
}

pub fn build_firmware_metadata(len: usize, digest: &[u8]) -> FirmwareInfo {
    FirmwareInfo {
        sha256: hex_encode(digest),
        size: len as u64,
        uploaded_at: Some(now_rfc3339()),
        staged: false,
        current_slot: None,
        pending_slot: None,
        previous_slot: None,
        slot_state: None,
    }
}

fn is_octet_stream(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|value| {
        value.split(';').next().map_or(false, |part| {
            part.trim().eq_ignore_ascii_case("application/octet-stream")
        })
    })
}

#[cfg(not(target_os = "espidf"))]
pub fn handle_firmware_upload(
    _app_state: &Lazy<Mutex<AppState>>,
    mut req: Request<&mut EspHttpConnection>,
) -> Result<(), EspError> {
    if !is_octet_stream(req.header("Content-Type")) {
        return http::respond_error(
            req,
            400,
            "Firmware upload requires Content-Type: application/octet-stream",
        );
    }

    warn!("OTA upload not supported on this target");
    http::respond_error(req, 501, "OTA not supported on this target")
}

#[cfg(target_os = "espidf")]
pub fn handle_firmware_upload(
    app_state: &Lazy<Mutex<AppState>>,
    req: Request<&mut EspHttpConnection>,
) -> Result<(), EspError> {
    if !is_octet_stream(req.header("Content-Type")) {
        return http::respond_error(
            req,
            400,
            "Firmware upload requires Content-Type: application/octet-stream",
        );
    }

    handle_firmware_upload_impl(app_state, req)
}

#[cfg(target_os = "espidf")]
#[allow(clippy::too_many_lines)]
fn handle_firmware_upload_impl(
    app_state: &Lazy<Mutex<AppState>>,
    mut req: Request<&mut EspHttpConnection>,
) -> Result<(), EspError> {
    info!("Starting OTA firmware upload");
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];
    let mut total_bytes = 0usize;
    let mut magic_checked = false;

    let mut ota = match EspOta::new() {
        Ok(instance) => instance,
        Err(err) => {
            error!("OTA init failed: {err:?}");
            return http::respond_error(req, 500, "OTA initialization failed");
        }
    };

    let running_slot = ota.get_running_slot().ok();
    let pending_slot = ota.get_update_slot().ok();

    let mut update = match ota.initiate_update() {
        Ok(handle) => handle,
        Err(err) => {
            error!("OTA begin failed: {err:?}");
            return http::respond_error(req, 500, "Failed to begin OTA update");
        }
    };

    loop {
        let read = match req.read(&mut buffer) {
            Ok(count) => count,
            Err(err) => {
                error!("Failed to read upload: {err:?}");
                let _ = update.abort();
                return http::respond_error(req, 400, "Failed to read firmware file");
            }
        };

        if read == 0 {
            break;
        }

        let chunk = &buffer[..read];
        if !magic_checked {
            if chunk.first() != Some(&0xE9) {
                let _ = update.abort();
                return http::respond_error(req, 400, "Invalid firmware: missing ESP32 magic byte");
            }
            magic_checked = true;
        }

        total_bytes = total_bytes.saturating_add(read);
        if let Err(validation) = enforce_firmware_constraints(total_bytes) {
            if matches!(&validation, FirmwareValidationError::TooLarge(_)) {
                warn!(
                    "Firmware upload exceeded limit: {total_bytes} bytes (max {MAX_FIRMWARE_SIZE})"
                );
            }
            let _ = update.abort();
            return http::respond_error(req, 400, validation.message());
        }

        hasher.update(chunk);
        if let Err(err) = update.write(chunk) {
            error!("OTA write failed: {err:?}");
            let _ = update.abort();
            return http::respond_error(req, 500, "Failed to write firmware");
        }
    }

    if let Err(validation) = enforce_firmware_constraints(total_bytes) {
        let _ = update.abort();
        return http::respond_error(req, 400, validation.message());
    }

    if let Err(err) = update.complete() {
        error!("OTA complete failed: {err:?}");
        return http::respond_error(req, 500, "Failed to complete OTA update");
    }

    let digest = hasher.finalize();
    let mut metadata = build_firmware_metadata(total_bytes, &digest);
    metadata.staged = true;
    metadata.current_slot = running_slot
        .as_ref()
        .map(|slot| slot.label.as_str().to_owned());
    metadata.previous_slot.clone_from(&metadata.current_slot);
    metadata.pending_slot = pending_slot
        .as_ref()
        .map(|slot| slot.label.as_str().to_owned());
    metadata.slot_state = Some("unverified".into());

    {
        let mut state = app_state.lock().unwrap();
        state.firmware = Some(metadata.clone());
    }

    info!(
        "OTA upload successful ({} bytes, sha256={}), rebooting in 3 seconds",
        metadata.size, metadata.sha256
    );

    http::respond_json(
        req,
        200,
        &json!({
            "uploaded": true,
            "size": metadata.size,
            "rebootIn": 3,
        }),
    )?;

    thread::spawn(|| {
        thread::sleep(Duration::from_secs(3));
        unsafe {
            sys::esp_restart();
        }
    });

    Ok(())
}

pub fn handle_rollback(
    app_state: &Lazy<Mutex<AppState>>,
    req: Request<&mut EspHttpConnection>,
) -> Result<(), EspError> {
    #[cfg(not(target_os = "espidf"))]
    {
        return http::respond_error(req, 501, "Rollback not supported on this target");
    }

    #[cfg(target_os = "espidf")]
    {
        info!("Manual OTA rollback requested");

        let ota = match EspOta::new() {
            Ok(instance) => instance,
            Err(err) => {
                error!("Failed to obtain OTA handle for rollback: {err:?}");
                return http::respond_error(req, 500, "OTA service unavailable");
            }
        };

        let fallback_slot = ota
            .get_update_slot()
            .ok()
            .map(|slot| slot.label.as_str().to_owned());

        {
            let mut state = app_state.lock().unwrap();
            if let Some(info) = state.firmware.as_mut() {
                info.staged = false;
                info.slot_state = Some("rollback".into());
                info.pending_slot = None;
                info.previous_slot.clone_from(&fallback_slot);
            }
        }

        http::respond_json(
            req,
            202,
            &json!({
                "rollback": "scheduled",
                "targetSlot": fallback_slot,
                "rebootIn": 1,
            }),
        )?;

        drop(ota);

        thread::spawn(|| {
            thread::sleep(Duration::from_secs(1));
            match EspOta::new() {
                Ok(mut ota) => {
                    let err = ota.mark_running_slot_invalid_and_reboot();
                    error!("Rollback request failed: {err:?}");
                }
                Err(err) => error!("Failed to reacquire OTA handle for rollback: {err:?}"),
            }
        });

        Ok(())
    }
}

#[cfg(target_os = "espidf")]
pub fn handle_pending_ota_state() {
    let ota = match EspOta::new() {
        Ok(instance) => instance,
        Err(err) => {
            warn!("OTA inspector init failed: {err:?}");
            return;
        }
    };

    let running_slot = match ota.get_running_slot() {
        Ok(slot) => slot,
        Err(err) => {
            warn!("Unable to determine running OTA slot: {err:?}");
            return;
        }
    };

    let fallback_slot = ota.get_update_slot().ok();
    update_firmware_state_from_slots(http::app_state(), &running_slot, fallback_slot.as_ref());

    let pending_verification = matches!(running_slot.state, SlotState::Unverified);

    drop(ota);

    if pending_verification {
        info!("Current firmware slot pending verification; scheduling confirmation");
        thread::spawn(|| {
            thread::sleep(Duration::from_secs(10));
            match EspOta::new() {
                Ok(mut ota) => {
                    if let Err(err) = ota.mark_running_slot_valid() {
                        error!("Failed to confirm OTA slot as valid: {err:?}");
                        return;
                    }

                    if let Ok(slot) = ota.get_running_slot() {
                        let fallback = ota.get_update_slot().ok();
                        update_firmware_state_from_slots(
                            http::app_state(),
                            &slot,
                            fallback.as_ref(),
                        );
                    }
                }
                Err(err) => error!("Failed to reopen OTA inspector for confirmation: {err:?}"),
            }
        });
    }
}

#[cfg(not(target_os = "espidf"))]
pub fn handle_pending_ota_state() {}

#[cfg(target_os = "espidf")]
fn update_firmware_state_from_slots(
    app_state: &Lazy<Mutex<AppState>>,
    running: &Slot,
    fallback: Option<&Slot>,
) {
    let mut state = app_state.lock().unwrap();
    let mut metadata = state.firmware.clone().unwrap_or_else(|| FirmwareInfo {
        sha256: String::new(),
        size: 0,
        uploaded_at: None,
        staged: false,
        current_slot: None,
        pending_slot: None,
        previous_slot: None,
        slot_state: None,
    });

    metadata.current_slot = Some(running.label.as_str().to_owned());
    metadata.slot_state = Some(slot_state_label(running.state).to_string());

    if matches!(running.state, SlotState::Unverified) {
        metadata.staged = true;
        if metadata.pending_slot.is_none() {
            metadata.pending_slot = fallback.map(|slot| slot.label.as_str().to_owned());
        }
    } else {
        metadata.staged = false;
        metadata.pending_slot = None;
    }

    metadata.previous_slot = fallback.map(|slot| slot.label.as_str().to_owned());

    state.firmware = Some(metadata);
}

#[cfg(target_os = "espidf")]
const fn slot_state_label(state: SlotState) -> &'static str {
    match state {
        SlotState::Factory => "factory",
        SlotState::Valid => "valid",
        SlotState::Invalid => "invalid",
        SlotState::Unverified => "unverified",
        SlotState::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn metadata_reports_expected_hash_and_size() {
        let payload = b"test firmware bytes";
        let mut hasher = sha2::Sha256::new();
        hasher.update(payload);
        let digest = hasher.finalize();

        let metadata = super::build_firmware_metadata(payload.len(), &digest);

        assert_eq!(metadata.size, payload.len() as u64);
        assert_eq!(metadata.sha256, crate::util::hex_encode(&digest));
        assert!(metadata.uploaded_at.is_some());
        assert!(!metadata.staged);
        assert!(metadata.current_slot.is_none());
        assert!(metadata.pending_slot.is_none());
        assert!(metadata.previous_slot.is_none());
        assert!(metadata.slot_state.is_none());
    }

    #[test]
    fn firmware_constraints_enforce_limits() {
        assert_eq!(
            super::enforce_firmware_constraints(0),
            Err(super::FirmwareValidationError::Empty)
        );

        let oversized = super::MAX_FIRMWARE_SIZE + 1;
        assert_eq!(
            super::enforce_firmware_constraints(oversized),
            Err(super::FirmwareValidationError::TooLarge(oversized))
        );

        assert_eq!(
            super::enforce_firmware_constraints(super::MAX_FIRMWARE_SIZE),
            Ok(())
        );
    }
}
