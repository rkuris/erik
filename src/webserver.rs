use std::{fmt::Write as _, sync::Mutex};

use esp_idf_hal::io::Write;
use esp_idf_svc::{
    http::{self, server::EspHttpServer, Method},
    wifi::{BlockingWifi, EspWifi},
};
use log::info;

use crate::{INTERNAL_TEMP_KEY, LATEST_TEMPS, RELAY_STATE};

pub fn create_server(
    _wifi: &mut BlockingWifi<EspWifi<'static>>,
) -> anyhow::Result<EspHttpServer<'static>> {
    let conf = http::server::Configuration::default();
    EspHttpServer::new(&conf).map_err(Into::into)
}

pub fn setup_http_handler(server: &mut EspHttpServer<'static>) -> anyhow::Result<()> {
    server.fn_handler("/", Method::Get, move |req| {
        // Extract the query string from the URI
        let uri = req.uri();
        let relay_param = extract_relay_param(uri);

        if let Some(query_param) = relay_param {
            info!("Received relay command: {query_param}");
            let relay_state = match query_param {
                "on" => true,
                "off" => false,
                _ => {
                    // Return a 400 Bad Request with an error message
                    return req
                        .into_status_response(400)?
                        .write_all(b"Invalid relay command");
                }
            };
            info!(
                "Setting relay to {}",
                if relay_state { "ON" } else { "OFF" }
            );
            *RELAY_STATE
                .get_or_init(|| Mutex::new(false))
                .lock()
                .unwrap() = relay_state;
        }

        let others = build_sensors_json();
        let relay_state = get_current_relay_state();

        // Get internal temperature from global storage
        let internal_temp = get_internal_temperature();

        req.into_ok_response()?.write_all(
            format!(
                r#"{{
                        "units": "farenheit",
                        "relay": {relay_state},
                        "sensors": {{
                            "internal": {internal_temp}
                            {others}
                        }}
                    }}"#,
            )
            .as_bytes(),
        )
    })?;

    Ok(())
}

fn extract_relay_param(uri: &str) -> Option<&str> {
    uri.split_once('?').map(|x| x.1).and_then(|qs| {
        qs.split('&').find_map(|kv| {
            let mut parts = kv.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some("relay"), Some(value)) => Some(value),
                _ => None,
            }
        })
    })
}

fn build_sensors_json() -> String {
    LATEST_TEMPS
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .iter()
        .filter(|(addr, _)| **addr != INTERNAL_TEMP_KEY) // Exclude internal sensor
        .map(|(addr, &temp)| {
            (
                // convert the addr to X:X:X:X:X:X:X:X
                addr.iter().map(u8::to_string).collect::<Vec<_>>().join(":"),
                temp,
            )
        })
        .fold(String::new(), |mut output, (addr, temp): (String, f64)| {
            // leading comma is safe here because we always have an internal
            // temperature first
            let _ = write!(output, r#", "{addr}": {temp}"#);
            output
        })
}

fn get_current_relay_state() -> bool {
    *RELAY_STATE
        .get()
        .expect("lock was crated earlier")
        .lock()
        .unwrap()
}

fn get_internal_temperature() -> f64 {
    LATEST_TEMPS
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .get(&INTERNAL_TEMP_KEY)
        .copied()
        .unwrap_or(0.0) // Default to 0.0 if not available yet
}
