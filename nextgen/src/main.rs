//! Next-generation firmware scaffold for the solar pool heater controller.

use std::{
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use esp_idf_svc::{
    http::server::{self, EspHttpConnection, EspHttpServer, Method, Request},
    io::{Read, Write},
    log::EspLogger,
    nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault},
    sys::{self, EspError, ESP_ERR_NVS_NOT_FOUND},
};
use include_dir::{include_dir, Dir};
use log::{error, info, warn};
use once_cell::sync::Lazy;
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::json;
use sha2::Sha256;
use time::OffsetDateTime;
use rand::{rngs::OsRng, RngCore};
use subtle::{Choice, ConstantTimeEq};
use sha2::{Sha256, Digest};

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

static WEB_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/webui");
static APP_STATE: Lazy<Mutex<AppState>> = Lazy::new(|| Mutex::new(AppState::default()));
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);
static HTTP_CONF: Lazy<server::Configuration> = Lazy::new(|| server::Configuration {
    stack_size: 16_384,
    ..server::Configuration::default()
});
const SALT_LEN: usize = 16;
const TOKEN_LEN: usize = 32;
const PBKDF2_ITERATIONS: u32 = 100_000;
const NVS_NAMESPACE: &str = "controller";
const NVS_KEY_USERNAME: &str = "user";
const NVS_KEY_PASSWORD_HASH: &str = "pwd_hash";
const NVS_KEY_SALT: &str = "pwd_salt";
const NVS_KEY_PROVISIONED: &str = "prov";
const MAX_NVS_STR_LEN: usize = 128;
const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MAX_FIRMWARE_SIZE: usize = 2 * 1024 * 1024;
const SESSION_EXPIRED_HEADER: &str = "X-Session-Expired";
const WWW_AUTH_INVALID: &str = "Bearer error=\"invalid_token\"";
const WWW_AUTH_EXPIRED: &str =
    "Bearer error=\"invalid_token\", error_description=\"session expired\"";

static NVS: Lazy<Mutex<EspNvs<NvsDefault>>> = Lazy::new(|| {
    let partition = EspDefaultNvsPartition::take().expect("Failed to take default NVS partition");
    let nvs = EspNvs::new(partition, NVS_NAMESPACE, true).expect("Failed to open NVS namespace");
    Mutex::new(nvs)
});

fn main() -> Result<()> {
    sys::link_patches();
    EspLogger::initialize_default();

    info!("Bootstrapping next-generation firmware scaffold");

    let _server = start_http_server()?;
    info!("HTTP server started; serving embedded web UI");

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

fn start_http_server() -> Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(&HTTP_CONF)?;

    // Static assets
    server.fn_handler("/", Method::Get, |req| {
        serve_static(req, "index.html", "text/html")
    })?;
    server.fn_handler("/index.html", Method::Get, |req| {
        serve_static(req, "index.html", "text/html")
    })?;
    server.fn_handler("/styles.css", Method::Get, |req| {
        serve_static(req, "styles.css", "text/css")
    })?;
    server.fn_handler("/app.js", Method::Get, |req| {
        serve_static(req, "app.js", "application/javascript")
    })?;

    // Provisioning (unauthenticated during first boot)
    server.fn_handler("/api/provisioning", Method::Get, |req| {
        handle_get_provisioning(req)
    })?;
    server.fn_handler(
        "/api/provisioning",
        Method::Post,
        |mut req| match parse_json::<ProvisioningRequest>(&mut req) {
            Ok(body) => handle_post_provisioning(req, body),
            Err(err) => respond_error(req, 400, &format!("Invalid provisioning payload: {err}")),
        },
    )?;

    // Auth
    server.fn_handler("/api/login", Method::Post, |mut req| {
        match parse_json::<LoginRequest>(&mut req) {
            Ok(body) => handle_login(req, body),
            Err(err) => respond_error(req, 400, &format!("Invalid login payload: {err}")),
        }
    })?;
    server.fn_handler("/api/logout", Method::Post, |req| handle_logout(req))?;

    // Status and controls
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
            Ok(body) => handle_relay(req, body),
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
            Ok(body) => handle_set_defaults(req, body),
            Err(err) => respond_error(req, 400, &format!("Invalid defaults payload: {err}")),
        }
    })?;
    server.fn_handler("/api/probes", Method::Get, |req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_get_probes(req)
    })?;

    // Wi-Fi provisioning stubs
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
            Ok(body) => handle_wifi_save(req, body),
            Err(err) => respond_error(req, 400, &format!("Invalid Wi-Fi payload: {err}")),
        }
    })?;

    // Admin endpoints
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
            Ok(body) => handle_password_change(req, body),
            Err(err) => respond_error(req, 400, &format!("Invalid password payload: {err}")),
        }
    })?;
    server.fn_handler("/api/admin/firmware", Method::Post, |mut req| {
        if let Err(failure) = authorize(&req) {
            return respond_unauthorized(req, failure);
        }
        handle_firmware_upload(req)
    })?;

    Ok(server)
}

fn serve_static(
    mut req: Request<&mut EspHttpConnection>,
    path: &str,
    content_type: &str,
) -> Result<(), EspError> {
    match WEB_ASSETS.get_file(path) {
        Some(file) => {
            let mut response = req.into_response(200, Some(content_type), &[])?;
            response.write_all(file.contents())?
        }
        None => {
            warn!("Static asset missing: {path}");
            let mut response = req.into_response(404, Some("text/plain"), &[])?;
            response.write_all(b"Not found")?
        }
    }
    Ok(())
}

fn handle_get_provisioning(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let state = APP_STATE.lock().unwrap();
    let response = ProvisioningStatusResponse {
        provisioned: state.provisioned,
        username: state
            .provisioned
            .then(|| state.credentials.username.clone()),
    };
    respond_json(req, 200, &response)
}

fn handle_post_provisioning(
    mut req: Request<&mut EspHttpConnection>,
    body: ProvisioningRequest,
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
    let previous_provisioned = state.provisioned;
    state.credentials = Credentials::with_password(&username, &body.password);
    state.provisioned = true;
    if let Err(err) = persist_credentials_state(&state.credentials, state.provisioned) {
        error!("Failed to persist credentials: {err}");
        state.credentials = previous_credentials;
        state.provisioned = previous_provisioned;
        return respond_error(req, 500, "Failed to persist credentials");
    }
    let token = state.credentials.issue_token();
    let response = json!({
        "provisioned": true,
        "token": token,
        "username": username,
        "expiresInSeconds": SESSION_IDLE_TIMEOUT.as_secs(),
    });
    respond_json(req, 200, &response)
}

fn handle_login(
    mut req: Request<&mut EspHttpConnection>,
    body: LoginRequest,
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
    respond_json(
        req,
        200,
        &json!({
            "token": token,
            "expiresInSeconds": SESSION_IDLE_TIMEOUT.as_secs(),
        }),
    )
}

fn handle_logout(mut req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    if let Err(failure) = authorize(&req) {
        return respond_unauthorized(req, failure);
    }
    let mut state = APP_STATE.lock().unwrap();
    state.credentials.invalidate_token();
    respond_empty(req, 204)
}

fn handle_status(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let state = APP_STATE.lock().unwrap();
    let response = StatusResponse {
        wifi: state.wifi.clone(),
        relay: state.relay.clone(),
        probes: state.probes.clone(),
        uptime_seconds: START_TIME.elapsed().as_secs(),
        firmware: state.firmware.clone(),
    };
    respond_json(req, 200, &response)
}

fn handle_relay(
    mut req: Request<&mut EspHttpConnection>,
    body: RelayRequest,
) -> Result<(), EspError> {
    let mut state = APP_STATE.lock().unwrap();
    match body.state.as_str() {
        "on" | "off" => {
            state.relay.state = body.state;
            state.relay.last_change = Some(now_rfc3339());
            respond_json(req, 200, &state.relay)
        }
        _ => respond_error(req, 400, "Relay state must be 'on' or 'off'"),
    }
}

fn handle_get_defaults(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let state = APP_STATE.lock().unwrap();
    respond_json(req, 200, &state.defaults)
}

fn handle_set_defaults(
    mut req: Request<&mut EspHttpConnection>,
    body: DefaultsRequest,
) -> Result<(), EspError> {
    let mut state = APP_STATE.lock().unwrap();
    state.defaults.default_state = body.default_state;
    state.defaults.hysteresis = body.hysteresis;
    state.defaults.min_on_temp = body.min_on_temp;
    respond_json(req, 200, &state.defaults)
}

fn handle_get_probes(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let state = APP_STATE.lock().unwrap();
    respond_json(req, 200, &state.probes)
}

fn handle_wifi_scan(req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    let response = WifiScanResponse {
        networks: vec![
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
    };
    respond_json(req, 200, &response)
}

fn handle_wifi_save(
    mut req: Request<&mut EspHttpConnection>,
    body: WifiSaveRequest,
) -> Result<(), EspError> {
    if body.ssid.trim().is_empty() {
        return respond_error(req, 400, "SSID cannot be empty");
    }
    let mut state = APP_STATE.lock().unwrap();
    state.wifi.mode = "STA".into();
    state.wifi.connected = false;
    state.wifi.ssid = body.ssid.clone();
    state.wifi.rssi = None;
    state.wifi.ip = None;
    respond_json(req, 200, &json!({"saved": true}))
}

fn handle_reboot(mut req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    info!("Reboot requested (stub)");
    respond_json(req, 200, &json!({"rebooting": true}))
}

fn handle_factory_reset(mut req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    info!("Factory reset requested (stub)");
    if let Err(err) = clear_persistent_credentials() {
        error!("Failed to clear persisted credentials: {err}");
        return respond_error(req, 500, "Failed to clear persisted credentials");
    }
    *APP_STATE.lock().unwrap() = AppState::default();
    respond_json(req, 200, &json!({"reset": true}))
}

fn enforce_firmware_constraints(len: usize) -> Result<(), FirmwareValidationError> {
    if len == 0 {
        return Err(FirmwareValidationError::Empty);
    }
    if len > MAX_FIRMWARE_SIZE {
        return Err(FirmwareValidationError::TooLarge(len));
    }
    Ok(())
}

fn build_firmware_metadata(len: usize, digest: &[u8]) -> FirmwareInfo {
    FirmwareInfo {
        sha256: hex_encode(digest),
        size: len as u64,
        uploaded_at: Some(now_rfc3339()),
        staged: false,
    }
}

fn is_octet_stream(content_type: Option<&str>) -> bool {
    content_type
        .and_then(|value| value.split(';').next())
        .map(|value| {
            value
                .trim()
                .eq_ignore_ascii_case("application/octet-stream")
        })
        .unwrap_or(false)
}

fn handle_firmware_upload(mut req: Request<&mut EspHttpConnection>) -> Result<(), EspError> {
    if !is_octet_stream(req.header("Content-Type")) {
        return respond_error(
            req,
            400,
            "Firmware upload requires Content-Type: application/octet-stream",
        );
    }

    let mut payload = Vec::new();
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];

    loop {
        let read = req.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        hasher.update(chunk);
        payload.extend_from_slice(chunk);
        if let Err(error @ FirmwareValidationError::TooLarge(_)) =
            enforce_firmware_constraints(payload.len())
        {
            warn!(
                "Firmware upload exceeded limit: {} bytes (max {})",
                payload.len(),
                MAX_FIRMWARE_SIZE
            );
            return respond_error(req, 400, error.message());
        }
    }

    if let Err(error) = enforce_firmware_constraints(payload.len()) {
        return respond_error(req, 400, error.message());
    }

    let digest = hasher.finalize();
    let metadata = build_firmware_metadata(payload.len(), &digest);

    info!(
        "Firmware upload stub received {} bytes (sha256={})",
        metadata.size, metadata.sha256
    );

    {
        let mut state = APP_STATE.lock().unwrap();
        state.firmware = Some(metadata.clone());
    }

    respond_json(req, 200, &metadata)
}

fn handle_password_change(
    mut req: Request<&mut EspHttpConnection>,
    body: PasswordChangeRequest,
) -> Result<(), EspError> {
    if body.new_password.trim().is_empty() {
        return respond_error(req, 400, "New password cannot be empty");
    }
    let mut state = APP_STATE.lock().unwrap();
    if !state.credentials.verify_password(&body.current_password) {
        return respond_error(req, 401, "Current password incorrect");
    }
    let previous = state.credentials.clone();
    state.credentials.set_password(&body.new_password);
    state.credentials.invalidate_token();
    if let Err(err) = persist_credentials_state(&state.credentials, state.provisioned) {
        error!("Failed to persist updated credentials: {err}");
        state.credentials = previous;
        return respond_error(req, 500, "Failed to persist credentials");
    }
    respond_json(req, 200, &json!({"changed": true}))
}

fn respond_json<T: Serialize>(
    req: Request<&mut EspHttpConnection>,
    status: u16,
    value: &T,
) -> Result<(), EspError> {
    match serde_json::to_vec(value) {
        Ok(body) => {
            let mut response = req.into_response(status, Some("application/json"), &[])?;
            response.write_all(&body)?;
            Ok(())
        }
        Err(err) => {
            error!("Serialization error: {err}");
            let mut response = req.into_response(500, Some("text/plain"), &[])?;
            response.write_all(b"Serialization error")?;
            Ok(())
        }
    }
}

fn respond_error(
    mut req: Request<&mut EspHttpConnection>,
    status: u16,
    message: &str,
) -> Result<(), EspError> {
    warn!("{}", message);
    let mut response = req.into_response(status, Some("text/plain"), &[])?;
    response.write_all(message.as_bytes())?;
    Ok(())
}

fn respond_empty(req: Request<&mut EspHttpConnection>, status: u16) -> Result<(), EspError> {
    let _ = req.into_response(status, None, &[])?;
    Ok(())
}

fn parse_json<T>(req: &mut Request<&mut EspHttpConnection>) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let mut buffer = Vec::new();
    let mut total_read = 0;
    loop {
        let read = req.read(&mut buffer[total_read..])?;
        if read == 0 { break; }
        total_read += read;
        if total_read >= buffer.len() { break; }
    }
    buffer.truncate(total_read);
    let parsed = serde_json::from_slice(&buffer)?;
    Ok(parsed)
}

fn now_rfc3339() -> String {
    let system_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos =
        (system_now.as_secs() as i128) * 1_000_000_000_i128 + system_now.subsec_nanos() as i128;
    let odt = OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .unwrap_or_else(|_| OffsetDateTime::UNIX_EPOCH);
    odt.to_string()
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

fn derive_password_hash(password: &str, salt: &[u8]) -> String {
    let mut output = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut output);
    hex_encode(&output)
}

fn constant_time_equals(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let choice: Choice = left.as_bytes().ct_eq(right.as_bytes());
    choice.unwrap_u8() == 1
}

fn read_nvs_str(nvs: &mut EspNvs<NvsDefault>, key: &str, buffer: &mut [u8]) -> Result<Option<String>> {
    match nvs.get_str(key, buffer) {
        Ok(Some(value)) => Ok(Some(value.to_owned())),
        Ok(None) => Ok(None),
        Err(err) if err.code() == ESP_ERR_NVS_NOT_FOUND => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn load_persistent_state() -> Result<Option<PersistentState>> {
    let mut nvs = NVS.lock().unwrap();
    let provisioned = match nvs.get_u8(NVS_KEY_PROVISIONED) {
        Ok(Some(value)) => value != 0,
        Ok(None) => false,
        Err(err) if err.code() == ESP_ERR_NVS_NOT_FOUND => false,
        Err(err) => return Err(err.into()),
    };

    if !provisioned {
        return Ok(None);
    }

    let mut buffer = [0u8; MAX_NVS_STR_LEN];
    let username = match read_nvs_str(&mut nvs, NVS_KEY_USERNAME, &mut buffer)? {
        Some(value) => value,
        None => return Ok(None),
    };

    let mut hash_buffer = [0u8; MAX_NVS_STR_LEN];
    let password_hash = match read_nvs_str(&mut nvs, NVS_KEY_PASSWORD_HASH, &mut hash_buffer)? {
        Some(value) => value,
        None => return Ok(None),
    };

    let mut salt_buffer = [0u8; MAX_NVS_STR_LEN];
    let salt_hex = match read_nvs_str(&mut nvs, NVS_KEY_SALT, &mut salt_buffer)? {
        Some(value) => value,
        None => return Ok(None),
    };

    let salt_vec = hex_decode(&salt_hex).map_err(|err| anyhow!("Invalid salt encoding: {err}"))?;
    if salt_vec.len() != SALT_LEN {
        return Err(anyhow!("Invalid salt length stored in NVS"));
    }
    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&salt_vec);

    Ok(Some(PersistentState {
        credentials: Credentials {
            username,
            password_hash,
            salt,
            token: None,
        },
        provisioned,
    }))
}

fn persist_credentials_state(credentials: &Credentials, provisioned: bool) -> Result<()> {
    let mut nvs = NVS.lock().unwrap();
    nvs.set_str(NVS_KEY_USERNAME, &credentials.username)?;
    nvs.set_str(NVS_KEY_PASSWORD_HASH, &credentials.password_hash)?;
    let salt_hex = hex_encode(&credentials.salt);
    nvs.set_str(NVS_KEY_SALT, &salt_hex)?;
    nvs.set_u8(NVS_KEY_PROVISIONED, if provisioned { 1 } else { 0 })?;
    nvs.commit()?;
    Ok(())
}

fn clear_persistent_credentials() -> Result<()> {
    let mut nvs = NVS.lock().unwrap();
    let _ = nvs.remove(NVS_KEY_USERNAME);
    let _ = nvs.remove(NVS_KEY_PASSWORD_HASH);
    let _ = nvs.remove(NVS_KEY_SALT);
    nvs.set_u8(NVS_KEY_PROVISIONED, 0)?;
    nvs.commit()?;
    Ok(())
}

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

enum AuthorizationFailure {
    Missing,
    Invalid,
    Expired,
}

#[derive(Debug, PartialEq, Eq)]
enum FirmwareValidationError {
    Empty,
    TooLarge(usize),
}

impl FirmwareValidationError {
    fn message(&self) -> &'static str {
        match self {
            FirmwareValidationError::Empty => "Firmware image cannot be empty",
            FirmwareValidationError::TooLarge(_) => "Firmware image exceeds 2 MiB limit",
        }
    }
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
    mut req: Request<&mut EspHttpConnection>,
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

    let mut response = req.into_response(401, Some("text/plain"), &headers)?;
    response.write_all(message.as_bytes())?;
    Ok(())
}

#[derive(Clone, Serialize)]
struct StatusResponse {
    wifi: WifiStatus,
    relay: RelayStatus,
    probes: Vec<ProbeInfo>,
    #[serde(rename = "uptimeSeconds")]
    uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    firmware: Option<FirmwareInfo>,
}

#[derive(Serialize)]
struct ProvisioningStatusResponse {
    provisioned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
}

#[derive(Clone, Serialize)]
struct WifiStatus {
    mode: String,
    ssid: String,
    connected: bool,
    rssi: Option<i32>,
    ip: Option<String>,
}

#[derive(Clone, Serialize)]
struct RelayStatus {
    state: String,
    #[serde(rename = "lastChange")]
    last_change: Option<String>,
}

#[derive(Clone, Serialize)]
struct ProbeInfo {
    id: String,
    name: Option<String>,
    fahrenheit: Option<f32>,
    #[serde(rename = "lastUpdated")]
    last_updated: Option<String>,
    enabled: bool,
}

#[derive(Clone, Serialize)]
struct Defaults {
    default_state: String,
    hysteresis: u16,
    #[serde(rename = "min_on_temp")]
    min_on_temp: u16,
}

#[derive(Clone)]
struct Credentials {
    username: String,
    password_hash: String,
    salt: [u8; SALT_LEN],
    token: Option<SessionToken>,
}

#[derive(Clone, Serialize)]
struct WifiNetwork {
    ssid: String,
    rssi: i32,
    secure: bool,
}

#[derive(Clone, Serialize)]
struct WifiScanResponse {
    networks: Vec<WifiNetwork>,
}

#[derive(Clone, Serialize)]
struct FirmwareInfo {
    sha256: String,
    size: u64,
    #[serde(rename = "uploadedAt")]
    uploaded_at: Option<String>,
    staged: bool,
}

#[derive(Clone)]
struct SessionToken {
    value: String,
    issued_at: SystemTime,
    last_seen: SystemTime,
}

impl SessionToken {
    fn new(value: String, now: SystemTime) -> Self {
        Self {
            value,
            issued_at: now,
            last_seen: now,
        }
    }

    fn is_expired(&self, now: SystemTime) -> bool {
        match now.duration_since(self.last_seen) {
            Ok(elapsed) => elapsed >= SESSION_IDLE_TIMEOUT,
            Err(_) => false,
        }
    }

    fn touch(&mut self, now: SystemTime) {
        self.last_seen = now;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenValidation {
    Authorized,
    Expired,
    Invalid,
}

struct AppState {
    wifi: WifiStatus,
    relay: RelayStatus,
    defaults: Defaults,
    probes: Vec<ProbeInfo>,
    firmware: Option<FirmwareInfo>,
    credentials: Credentials,
    provisioned: bool,
}

struct PersistentState {
    credentials: Credentials,
    provisioned: bool,
}

impl Default for WifiStatus {
    fn default() -> Self {
        Self {
            mode: "AP".into(),
            ssid: "Solar-Heater".into(),
            connected: false,
            rssi: None,
            ip: Some("192.168.4.1".into()),
        }
    }
}

impl Default for RelayStatus {
    fn default() -> Self {
        Self {
            state: "off".into(),
            last_change: None,
        }
    }
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

impl Default for Credentials {
    fn default() -> Self {
        Self::with_password("admin", "admin")
    }
}

impl Credentials {
    fn with_password(username: &str, password: &str) -> Self {
        let salt = generate_salt();
        let password_hash = derive_password_hash(password, &salt);
        Self {
            username: username.into(),
            password_hash,
            salt,
            token: None,
        }
    }

    fn verify_password(&self, candidate: &str) -> bool {
        let candidate_hash = derive_password_hash(candidate, &self.salt);
        constant_time_equals(&candidate_hash, &self.password_hash)
    }

    fn set_password(&mut self, new_password: &str) {
        self.salt = generate_salt();
        self.password_hash = derive_password_hash(new_password, &self.salt);
    }

    fn issue_token(&mut self) -> String {
        let token_value = generate_token();
        let now = SystemTime::now();
        self.token = Some(SessionToken::new(token_value.clone(), now));
        token_value
    }

    fn invalidate_token(&mut self) {
        self.token = None;
    }

    fn validate_token(&mut self, candidate: &str, now: SystemTime) -> TokenValidation {
        match self.token {
            Some(ref mut session) => {
                if !constant_time_equals(&session.value, candidate) {
                    return TokenValidation::Invalid;
                }
                if session.is_expired(now) {
                    self.invalidate_token();
                    TokenValidation::Expired
                } else {
                    session.touch(now);
                    TokenValidation::Authorized
                }
            }
            None => TokenValidation::Invalid,
        }
    }
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
            wifi: WifiStatus::default(),
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

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RelayRequest {
    state: String,
}

#[derive(Deserialize)]
struct DefaultsRequest {
    default_state: String,
    hysteresis: u16,
    min_on_temp: u16,
}

#[derive(Deserialize)]
struct WifiSaveRequest {
    ssid: String,
    password: Option<String>,
}

#[derive(Deserialize)]
struct PasswordChangeRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct ProvisioningRequest {
    username: Option<String>,
    password: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::time::Duration;

    #[test]
    fn session_tokens_expire_after_idle_timeout() {
        let mut credentials = Credentials::with_password("tester", "password123");
        let token_value = credentials.issue_token();

        if let Some(ref mut token) = credentials.token {
            token.last_seen = token
                .last_seen
                .checked_sub(SESSION_IDLE_TIMEOUT + Duration::from_secs(1))
                .expect("time underflow");
        }

        let result = credentials.validate_token(&token_value, SystemTime::now());
        assert_eq!(result, TokenValidation::Expired);
        assert!(credentials.token.is_none());
    }

    #[test]
    fn session_tokens_touch_on_activity() {
        let mut credentials = Credentials::with_password("tester", "password123");
        let token_value = credentials.issue_token();
        let probe_time = SystemTime::now()
            .checked_add(Duration::from_secs(60))
            .expect("time overflow");

        let result = credentials.validate_token(&token_value, probe_time);
        assert_eq!(result, TokenValidation::Authorized);

        let last_seen = credentials
            .token
            .as_ref()
            .expect("token should remain active")
            .last_seen;
        assert!(last_seen.duration_since(probe_time).is_ok());
    }

    #[test]
    fn firmware_constraints_enforce_limits() {
        assert_eq!(
            enforce_firmware_constraints(0),
            Err(FirmwareValidationError::Empty)
        );

        let oversized = MAX_FIRMWARE_SIZE + 1;
        assert_eq!(
            enforce_firmware_constraints(oversized),
            Err(FirmwareValidationError::TooLarge(oversized))
        );

        assert_eq!(enforce_firmware_constraints(MAX_FIRMWARE_SIZE), Ok(()));
    }

    #[test]
    fn metadata_reports_expected_hash_and_size() {
        let payload = b"test firmware bytes";
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let digest = hasher.finalize();

        let metadata = build_firmware_metadata(payload.len(), &digest);

        assert_eq!(metadata.size, payload.len() as u64);
        assert_eq!(metadata.sha256, hex_encode(&digest));
        assert!(metadata.uploaded_at.is_some());
        assert!(!metadata.staged);
    }
}
