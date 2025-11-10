use std::{sync::Mutex, thread, time::{Duration, Instant, SystemTime}};

use anyhow::Result;
use esp_idf_svc::{
    http::server::{self, EspHttpConnection, EspHttpServer, Method, Request},
    io::{EspIOError, Write},
    sys::{self, EspError},
};
use log::{error, info, warn};
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;

use crate::{
    api::{
        ApInfo, Defaults, DefaultsRequest, FirmwareInfo, LoginRequest, PasswordChangeRequest,
        ProbeInfo, ProvisioningRequest, ProvisioningStatusResponse, RelayRequest, RelayStatus,
        StatusResponse, WifiSaveRequest, WifiStatus,
    },
    auth::{Credentials, TokenValidation},
    ota,
    storage::{
        self, clear_persistent_credentials, clear_wifi_credentials, load_persistent_state,
        persist_credentials_state, persist_wifi_credentials,
    },
    util::now_rfc3339,
    wifi::{self, ProvisioningState},
    MAX_FIRMWARE_SIZE, SESSION_IDLE_TIMEOUT,
};

struct EmbeddedAsset {
    name: &'static str,
    mime: &'static str,
    data: &'static [u8],
}

const EMBEDDED_ASSETS: &[EmbeddedAsset] = &[
    EmbeddedAsset {
        name: "index.html",
        mime: "text/html",
        data: include_bytes!("../webui/index.html"),
    },
    EmbeddedAsset {
        name: "styles.css",
        mime: "text/css",
        data: include_bytes!("../webui/styles.css"),
    },
    EmbeddedAsset {
        name: "app.js",
        mime: "application/javascript",
        data: include_bytes!("../webui/app.js"),
    },
    EmbeddedAsset {
        name: "favicon.svg",
        mime: "image/svg+xml",
        data: include_bytes!("../webui/favicon.svg"),
    },
];
static HTTP_CONF: Lazy<server::Configuration> = Lazy::new(|| server::Configuration {
    stack_size: 16_384,
    ..server::Configuration::default()
});
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);
static APP_STATE: Lazy<Mutex<AppState>> = Lazy::new(|| Mutex::new(AppState::default()));

const SESSION_EXPIRED_HEADER: &str = "X-Session-Expired";
const WWW_AUTH_INVALID: &str = "Bearer error=\"invalid_token\"";
const WWW_AUTH_EXPIRED: &str =
    "Bearer error=\"invalid_token\", error_description=\"session expired\"";

pub fn start_http_server() -> Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(&HTTP_CONF)?;

    register_static_routes(&mut server)?;
    register_provisioning_routes(&mut server)?;
    register_auth_routes(&mut server)?;
    register_status_routes(&mut server)?;
    register_wifi_routes(&mut server)?;
    register_admin_routes(&mut server)?;

    Ok(server)
}

fn register_static_routes(server: &mut EspHttpServer<'static>) -> Result<(), EspError> {
    server.fn_handler("/", Method::Get, |req| serve_static(req, "index.html", "text/html"))?;
    server.fn_handler("/index.html", Method::Get, |req| {
        serve_static(req, "index.html", "text/html")
    })?;
    server.fn_handler("/styles.css", Method::Get, |req| {
        serve_static(req, "styles.css", "text/css")
    })?;
    server.fn_handler("/app.js", Method::Get, |req| {
        serve_static(req, "app.js", "application/javascript")
    })?;
    server.fn_handler("/favicon.svg", Method::Get, |req| {
        serve_static(req, "favicon.svg", "image/svg+xml")
    })?;
    Ok(())
}

fn register_provisioning_routes(server: &mut EspHttpServer<'static>) -> Result<(), EspError> {
    server.fn_handler("/api/provisioning", Method::Get, handle_get_provisioning)?;
    server.fn_handler("/api/provisioning", Method::Post, |mut req| {
        match parse_json::<ProvisioningRequest>(&mut req) {
            Ok(body) => handle_post_provisioning(req, &body),
            Err(err) => respond_error(req, 400, &format!("Invalid provisioning payload: {err}")),
        }
    })?;
    Ok(())
}

fn register_auth_routes(server: &mut EspHttpServer<'static>) -> Result<(), EspError> {
    server.fn_handler("/api/login", Method::Post, |mut req| {
        match parse_json::<LoginRequest>(&mut req) {
            Ok(body) => handle_login(req, &body),
            Err(err) => respond_error(req, 400, &format!("Invalid login payload: {err}")),
        }
    })?;
    server.fn_handler("/api/logout", Method::Post, handle_logout)?;
    Ok(())
}

fn register_status_routes(server: &mut EspHttpServer<'static>) -> Result<(), EspError> {
    server.fn_handler("/api/status", Method::Get, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_status(req)
    })?;
    server.fn_handler("/api/relay", Method::Post, |mut req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        match parse_json::<RelayRequest>(&mut req) {
            Ok(body) => handle_relay(req, &body),
            Err(err) => respond_error(req, 400, &format!("Invalid relay payload: {err}")),
        }
    })?;
    server.fn_handler("/api/defaults", Method::Get, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_get_defaults(req)
    })?;
    server.fn_handler("/api/defaults", Method::Post, |mut req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        match parse_json::<DefaultsRequest>(&mut req) {
            Ok(body) => handle_set_defaults(req, &body),
            Err(err) => respond_error(req, 400, &format!("Invalid defaults payload: {err}")),
        }
    })?;
    server.fn_handler("/api/probes", Method::Get, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_get_probes(req)
    })?;
    Ok(())
}

fn register_wifi_routes(server: &mut EspHttpServer<'static>) -> Result<(), EspError> {
    server.fn_handler("/api/wifi/scan", Method::Get, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_wifi_scan(req)
    })?;
    server.fn_handler("/api/wifi", Method::Post, |mut req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        match parse_json::<WifiSaveRequest>(&mut req) {
            Ok(body) => handle_wifi_save(req, &body),
            Err(err) => respond_error(req, 400, &format!("Invalid Wi-Fi payload: {err}")),
        }
    })?;
    Ok(())
}

fn register_admin_routes(server: &mut EspHttpServer<'static>) -> Result<(), EspError> {
    server.fn_handler("/api/admin/reboot", Method::Post, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_reboot(req)
    })?;
    server.fn_handler("/api/admin/factory-reset", Method::Post, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_factory_reset(req)
    })?;
    server.fn_handler("/api/admin/password", Method::Post, |mut req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        match parse_json::<PasswordChangeRequest>(&mut req) {
            Ok(body) => handle_password_change(req, &body),
            Err(err) => respond_error(req, 400, &format!("Invalid password payload: {err}")),
        }
    })?;
    server.fn_handler("/api/admin/firmware", Method::Post, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        ota::handle_firmware_upload(&APP_STATE, req)
    })?;
    server.fn_handler("/api/admin/rollback", Method::Post, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        ota::handle_rollback(&APP_STATE, req)
    })?;
    server.fn_handler("/api/firmware/info", Method::Get, |_req| handle_firmware_info(_req))?;
    Ok(())
}
fn handle_firmware_info(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    use crate::api::FirmwareDescriptor;
    let info = FirmwareDescriptor::default();
    respond_json(req, 200, &info)
}

pub fn app_state() -> &'static Lazy<Mutex<AppState>> {
    &APP_STATE
}

pub fn respond_json<T>(
    req: Request<&mut EspHttpConnection>,
    status: u16,
    value: &T,
) -> Result<(), EspError>
where
    T: Serialize,
{
    match serde_json::to_vec(value) {
        Ok(body) => {
            let mut response = req
                .into_response(status, Some("application/json"), &[])
                .map_err(io_error_to_esp)?;
            response.write_all(&body).map_err(io_error_to_esp)?;
            Ok(())
        }
        Err(err) => {
            error!("Serialization error: {err}");
            let mut response = req
                .into_response(500, Some("text/plain"), &[])
                .map_err(io_error_to_esp)?;
            response
                .write_all(b"Serialization error")
                .map_err(io_error_to_esp)?;
            Ok(())
        }
    }
}

pub fn respond_error(
    req: Request<&mut EspHttpConnection>,
    status: u16,
    message: &str,
) -> Result<(), EspError> {
    warn!("{message}");
    let mut response = req
        .into_response(status, Some("text/plain"), &[])
        .map_err(io_error_to_esp)?;
    response
        .write_all(message.as_bytes())
        .map_err(io_error_to_esp)?;
    Ok(())
}

fn respond_empty(
    req: Request<&mut EspHttpConnection>,
    status: u16,
) -> Result<(), EspError> {
    let _ = req.into_response(status, None, &[]).map_err(io_error_to_esp)?;
    Ok(())
}

fn parse_json<T>(req: &mut Request<&mut EspHttpConnection>) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    const READ_CHUNK: usize = 1024;
    let mut buffer = Vec::new();
    let mut chunk = [0u8; READ_CHUNK];

    loop {
        let read = req.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_FIRMWARE_SIZE {
            break;
        }
    }

    let parsed = serde_json::from_slice(&buffer)?;
    Ok(parsed)
}

fn authorize(req: &Request<&mut EspHttpConnection>) -> Result<(), AuthorizationFailure> {
    let header_value = req
        .header("Authorization")
        .ok_or(AuthorizationFailure::Missing)?;
    let token = extract_bearer_token(header_value).ok_or(AuthorizationFailure::Invalid)?;
    let mut state = APP_STATE.lock().unwrap();
    match state.credentials.validate_token(token, SystemTime::now()) {
        TokenValidation::Authorized => Ok(()),
        TokenValidation::Expired => Err(AuthorizationFailure::Expired),
        TokenValidation::Invalid => Err(AuthorizationFailure::Invalid),
    }
}

fn respond_unauthorized(
    req: Request<&mut EspHttpConnection>,
    failure: AuthorizationFailure,
) -> Result<(), EspError> {
    let (message, headers): (&str, Vec<(&str, &str)>) = match failure {
        AuthorizationFailure::Missing | AuthorizationFailure::Invalid => {
            ("Unauthorized", vec![("WWW-Authenticate", WWW_AUTH_INVALID)])
        }
        AuthorizationFailure::Expired => (
            "Session expired",
            vec![
                ("WWW-Authenticate", WWW_AUTH_EXPIRED),
                (SESSION_EXPIRED_HEADER, "1"),
            ],
        ),
    };

    let mut response = req
        .into_response(401, Some("text/plain"), &headers)
        .map_err(io_error_to_esp)?;
    response
        .write_all(message.as_bytes())
        .map_err(io_error_to_esp)?;
    Ok(())
}

fn provisioning_state_label(state: ProvisioningState) -> String {
    match state {
        ProvisioningState::Idle => "idle",
        ProvisioningState::Connecting => "connecting",
        ProvisioningState::ApMode => "ap-mode",
        ProvisioningState::Error => "error",
    }
    .to_string()
}

fn now_uptime_seconds() -> u64 {
    START_TIME.elapsed().as_secs()
}

pub struct AppState {
    pub relay: RelayStatus,
    pub defaults: Defaults,
    pub probes: Vec<ProbeInfo>,
    pub firmware: Option<FirmwareInfo>,
    pub credentials: Credentials,
    pub provisioned: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let (credentials, provisioned) = match load_persistent_state() {
            Ok(Some(state)) => (state.credentials, state.provisioned),
            Ok(None) => (Credentials::default(), false),
            Err(err) => {
                warn!("Failed to load persisted credentials: {err}");
                (Credentials::default(), false)
            }
        };

        Self {
            relay: RelayStatus::default(),
            defaults: Defaults::default(),
            probes: vec![
                ProbeInfo {
                    id: "28-00000abcd123".into(),
                    name: Some("Pool Return".into()),
                    fahrenheit: Some(74.8),
                    last_updated: Some(now_rfc3339()),
                    enabled: true,
                },
                ProbeInfo {
                    id: "28-00000abcd456".into(),
                    name: Some("Roof".into()),
                    fahrenheit: Some(102.9),
                    last_updated: Some(now_rfc3339()),
                    enabled: true,
                },
            ],
            firmware: None,
            credentials,
            provisioned,
        }
    }
}

fn serve_static(
    req: Request<&mut EspHttpConnection>,
    path: &str,
    _fallback_content_type: &str,
) -> Result<(), EspError> {
    if let Some(asset) = EMBEDDED_ASSETS.iter().find(|asset| asset.name == path) {
        let mut response = req
            .into_response(200, Some(asset.mime), &[])
            .map_err(io_error_to_esp)?;
        response
            .write_all(asset.data)
            .map_err(io_error_to_esp)?;
    } else {
        warn!("Static asset missing: {path}");
        let mut response = req
            .into_response(404, Some("text/plain"), &[])
            .map_err(io_error_to_esp)?;
        response
            .write_all(b"Not found")
            .map_err(io_error_to_esp)?;
    }
    Ok(())
}

fn handle_get_provisioning(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let response = {
        let state = APP_STATE.lock().unwrap();
        ProvisioningStatusResponse {
            provisioned: state.provisioned,
            username: state
                .provisioned
                .then(|| state.credentials.username.clone()),
        }
    };
    respond_json(req, 200, &response)
}

fn handle_post_provisioning(
    req: Request<&mut EspHttpConnection>,
    body: &ProvisioningRequest,
) -> Result<(), EspError> {
    let mut state = APP_STATE.lock().unwrap();
    if state.provisioned {
        return respond_error(req, 409, "Already provisioned");
    }

    let username = body
        .username
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("admin")
        .to_owned();

    if body.password.trim().len() < 8 {
        return respond_error(req, 400, "Password must be at least 8 characters");
    }

    let previous_credentials = state.credentials.clone();
    state.credentials = Credentials::with_password(&username, &body.password);
    state.provisioned = true;
    if let Err(err) = persist_credentials_state(&state.credentials, state.provisioned) {
        error!("Failed to persist credentials: {err}");
        state.credentials = previous_credentials;
        state.provisioned = false;
        return respond_error(req, 500, "Failed to persist credentials");
    }

    let token = state.credentials.issue_token();
    let response = json!({
        "provisioned": true,
        "token": token,
        "username": username,
        "expiresInSeconds": SESSION_IDLE_TIMEOUT.as_secs(),
    });
    drop(state);
    respond_json(req, 200, &response)
}

fn handle_login(
    req: Request<&mut EspHttpConnection>,
    body: &LoginRequest,
) -> Result<(), EspError> {
    let mut state = APP_STATE.lock().unwrap();
    if !state.provisioned {
        return respond_error(req, 423, "Provisioning required");
    }
    if body.username != state.credentials.username {
        return respond_error(req, 401, "Invalid credentials");
    }
    if !state.credentials.verify_password(&body.password) {
        return respond_error(req, 401, "Invalid credentials");
    }
    let token = state.credentials.issue_token();
    drop(state);
    respond_json(
        req,
        200,
        &json!({
            "token": token,
            "expiresInSeconds": SESSION_IDLE_TIMEOUT.as_secs(),
        }),
    )
}

fn handle_logout(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    APP_STATE
        .lock()
        .unwrap()
        .credentials
        .invalidate_token();
    respond_empty(req, 204)
}

fn handle_status(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let wifi_snapshot = wifi::CONTROLLER.snapshot();
    let provisioning_state = provisioning_state_label(wifi_snapshot.provisioning_state);
    let ap_info = wifi_snapshot.access_point.as_ref().map(|ap| ApInfo {
        ssid: ap.ssid.clone(),
        channel: ap.channel,
        client_count: ap.client_count,
    });

    let wifi_status = WifiStatus::from_snapshot(&wifi_snapshot);

    let response = {
        let state = APP_STATE.lock().unwrap();
        StatusResponse {
            wifi: wifi_status,
            relay: state.relay.clone(),
            probes: state.probes.clone(),
            uptime_seconds: now_uptime_seconds(),
            firmware: state.firmware.clone(),
            provisioning_state,
            ap: ap_info,
        }
    };
    respond_json(req, 200, &response)
}

fn handle_relay(
    req: Request<&mut EspHttpConnection>,
    body: &RelayRequest,
) -> Result<(), EspError> {
    let mut state = APP_STATE.lock().unwrap();
    let response = match body.state.as_str() {
        "on" | "off" => {
            state.relay.state.clear();
            state.relay.state.push_str(&body.state);
            state.relay.last_change = Some(now_rfc3339());
            state.relay.clone()
        }
        _ => return respond_error(req, 400, "Relay state must be 'on' or 'off'"),
    };
    drop(state);
    respond_json(req, 200, &response)
}

fn handle_get_defaults(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let defaults = {
        let state = APP_STATE.lock().unwrap();
        state.defaults.clone()
    };
    respond_json(req, 200, &defaults)
}

fn handle_set_defaults(
    req: Request<&mut EspHttpConnection>,
    body: &DefaultsRequest,
) -> Result<(), EspError> {
    let mut state = APP_STATE.lock().unwrap();
    state.defaults.default_state.clear();
    state
        .defaults
        .default_state
        .push_str(&body.default_state);
    state.defaults.hysteresis = body.hysteresis;
    state.defaults.min_on_temp = body.min_on_temp;
    let defaults = state.defaults.clone();
    drop(state);
    respond_json(req, 200, &defaults)
}

fn handle_get_probes(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let probes = {
        let state = APP_STATE.lock().unwrap();
        state.probes.clone()
    };
    respond_json(req, 200, &probes)
}

fn handle_wifi_scan(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let response = wifi::CONTROLLER.scan_networks();
    respond_json(req, 200, &response)
}

fn handle_wifi_save(
    req: Request<&mut EspHttpConnection>,
    body: &WifiSaveRequest,
) -> Result<(), EspError> {
    if body.ssid.trim().is_empty() {
        return respond_error(req, 400, "SSID cannot be empty");
    }

    let credentials = storage::WifiCredentials {
        ssid: body.ssid.clone(),
        password: body.password.clone(),
    };

    if let Err(err) = persist_wifi_credentials(&credentials) {
        error!("Failed to save Wi-Fi credentials: {err:?}");
        return respond_error(req, 500, "Failed to save credentials");
    }

    wifi::CONTROLLER.begin_sta_attempt(&body.ssid);

    respond_json(
        req,
        200,
        &json!({
            "saved": true,
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

fn handle_reboot(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    info!("Reboot requested (stub)");
    respond_json(req, 200, &json!({"rebooting": true}))
}

fn handle_factory_reset(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    info!("Factory reset requested (stub)");
    if let Err(err) = clear_persistent_credentials() {
        error!("Failed to clear persisted credentials: {err}");
        return respond_error(req, 500, "Failed to clear persisted credentials");
    }
    if let Err(err) = clear_wifi_credentials() {
        error!("Failed to clear Wi-Fi credentials: {err}");
        return respond_error(req, 500, "Failed to clear Wi-Fi credentials");
    }
    *APP_STATE.lock().unwrap() = AppState::default();
    wifi::CONTROLLER.enable_captive_ap(None);
    respond_json(req, 200, &json!({"reset": true}))
}

fn handle_password_change(
    req: Request<&mut EspHttpConnection>,
    body: &PasswordChangeRequest,
) -> Result<(), EspError> {
    if body.new_password.trim().is_empty() {
        return respond_error(req, 400, "New password cannot be empty");
    }
    let mut state = APP_STATE.lock().unwrap();
    if !state
        .credentials
        .verify_password(&body.current_password)
    {
        drop(state);
        return respond_error(req, 401, "Current password incorrect");
    }

    let previous = state.credentials.clone();
    state.credentials.set_password(&body.new_password);
    state.credentials.invalidate_token();

    let persist_result = persist_credentials_state(&state.credentials, state.provisioned);

    if let Err(err) = persist_result {
        state.credentials = previous;
        drop(state);
        error!("Failed to persist updated credentials: {err}");
        return respond_error(req, 500, "Failed to persist credentials");
    }

    drop(state);
    respond_json(req, 200, &json!({"changed": true}))
}

#[allow(clippy::missing_const_for_fn)]
fn extract_bearer_token(header_value: &str) -> Option<&str> {
    let mut parts = header_value.splitn(2, ' ');
    let scheme = parts.next()?.trim();
    let token = parts.next()?.trim();
    if scheme.eq_ignore_ascii_case("bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

const fn io_error_to_esp(err: EspIOError) -> EspError {
    err.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationFailure {
    Missing,
    Invalid,
    Expired,
}
