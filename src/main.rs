//! A ESP32C6 embedded solution for a pool temperature valve

mod secrets;

use anyhow::anyhow;
use esp_idf_hal::{
    delay::FreeRtos,
    gpio::PinDriver,
    io::Write as _,
    temp_sensor::{TempSensorConfig, TempSensorDriver},
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    http::{self, server::EspHttpServer, Method},
    log::{set_target_level, EspLogger},
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
    sys,
};
use log::{info, warn};
use onewire::OneWire;

include!("secrets.rs");

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    // only produce global warnings, but info for our own stuff
    EspLogger::initialize_default();
    set_target_level("*", log::LevelFilter::Warn)?;
    set_target_level("erik", log::LevelFilter::Info)?;

    info!("Starting up");

    // Get the resources we'll need later
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!("Setting up wifi");
    let espwifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(espwifi, sys_loop)?;
    while let Err(e) = connect_wifi(&mut wifi) {
        warn!("Error connecting to wifi: {e}, retrying in 10s");
        FreeRtos::delay_ms(10_000);
    }

    info!("Setting up the temperature sensor");
    let mut ts_driver =
        TempSensorDriver::new(&TempSensorConfig::default(), peripherals.temp_sensor)?;
    ts_driver.enable()?;
    info!("Temperature is {}F", ts_driver.get_fahrenheit()?);

    info!("Setting up a web server");
    let mut server = create_server(&mut wifi)?;
    server.fn_handler("/", Method::Get, move |req| {
        req.into_ok_response()?
            .write_all(
                format!(
                    r#"{{
                        "units": "farenheit",
                        "sensors": {{
                            "internal": "{}"
                        }}
                    }}"#,
                    ts_driver.get_fahrenheit().unwrap()
                )
                .as_bytes(),
            )
    })?;

    info!("Setting up the temperature probe");
    let mut pin = PinDriver::input_output_od(peripherals.pins.gpio10)?;
    let mut wire = OneWire::new(&mut pin, false);
    let mut delay = esp_idf_hal::delay::Delay::new(10000);
    if wire.reset(&mut delay).is_err() {
        warn!(
            "extern temperature probe reset failed on pin {:?}",
            pin.pin()
        );
    }

    info!("flashing the user LED");

    let mut led = PinDriver::output(peripherals.pins.gpio15)?;

    loop {
        led.toggle()?;
        FreeRtos::delay_ms(500);
    }

    /*
    std::mem::forget(wifi);
    std::mem::forget(server);

    Ok(()) */
}

fn create_server(
    _wifi: &mut BlockingWifi<EspWifi<'static>>,
) -> anyhow::Result<EspHttpServer<'static>> {
    let conf = http::server::Configuration::default();

    EspHttpServer::new(&conf).map_err(Into::into)
}

/// Connect to the first known wifi
fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let known_ssids = secrets::get();

    info!("configuring wifi");
    let wifi_configuration = Configuration::Client(ClientConfiguration {
        auth_method: AuthMethod::None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;

    info!("scanning for APs");

    let mut conf = None;
    for ap in wifi.scan().unwrap_or_default() {
        if let Some(&password) = known_ssids.get(ap.ssid.as_str()) {
            info!("Found known ssid '{}'", ap.ssid);
            conf = Some(ClientConfiguration {
                ssid: ap.ssid,
                password: password.try_into().unwrap(),
                ..Default::default()
            });
            break;
        }
        info!("Skipping unknown ssid '{}'", ap.ssid);
    }
    let Some(conf) = conf else {
        return Err(anyhow!("No known AP found"));
    };

    info!("Connecting to ssid '{}'", conf.ssid);

    wifi.set_configuration(&Configuration::Client(conf))?;
    wifi.connect()?;

    info!("connected");
    Ok(())
}
