//! A ESP32C6 embedded solution for a pool temperature valve

mod secrets;

use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::{Mutex, OnceLock},
};

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
    sys,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::{info, warn};
use onewire::{ds18b20, DeviceSearch, OneWire, DS18B20};

static LATEST_TEMPS: OnceLock<Mutex<HashMap<[u8; 8], f64>>> = OnceLock::new();

#[allow(clippy::too_many_lines)]
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

    info!("Setting up the temperature probe");
    let mut pin = PinDriver::input_output_od(peripherals.pins.gpio2)?;
    let mut wire = OneWire::new(&mut pin, false);
    let mut delay = esp_idf_hal::delay::Delay::new(10000);
    let mut sensors = HashMap::new();
    if wire.reset(&mut delay).is_err() {
        warn!("extern temperature probe reset failed");
    } else {
        let mut search = DeviceSearch::new();
        while let Ok(Some(device)) = wire.search_next(&mut search, &mut delay) {
            info!("found device {device:?}");
            let addr = device.address;
            if let Ok(ds18b20) = DS18B20::new(device) {
                info!("and it was a ds18b20");
                let resolution = ds18b20.measure_temperature(&mut wire, &mut delay).unwrap();
                delay.delay_ms(u32::from(resolution.time_ms()));
                let temperature = ds18b20
                    .read_temperature(&mut wire, &mut delay)
                    .map(ds18b20::split_temp)
                    .map(|(integral, fractions)| {
                        f64::from(integral) + f64::from(fractions) / 10000.0
                    })
                    .unwrap();
                info!("success! Temperature: {temperature}");
                sensors.insert(addr, ds18b20);
                LATEST_TEMPS
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                    .unwrap()
                    .insert(addr, temperature);
            } else {
                warn!("Device didn't seem to be a ds18b20");
            }
        }
    }

    info!("Setting up a web server");
    let mut server = create_server(&mut wifi)?;
    server.fn_handler("/", Method::Get, move |req| {
        let others = LATEST_TEMPS
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .iter()
            .map(|(addr, &temp)| {
                (
                    // convert the addr to X:X:X:X:X:X:X:X
                    addr.iter().map(u8::to_string).collect::<Vec<_>>().join(":"),
                    // convert the temperature into farenheight
                    temp * 9.0 / 5.0 + 32.0,
                )
            })
            .fold(String::new(), |mut output, (addr, temp): (String, f64)| {
                // leading comma is safe here because we always have an internal
                // temperature first
                let _ = write!(output, r#", "{addr}": {temp}"#);
                output
            });

        req.into_ok_response()?.write_all(
            format!(
                r#"{{
                        "units": "farenheit",
                        "sensors": {{
                            "internal": {}
                            `{}
                        }}
                    }}"#,
                ts_driver.get_fahrenheit().unwrap(),
                others,
            )
            .as_bytes(),
        )
    })?;

    info!("flashing the user LED");

    let mut led = PinDriver::output(peripherals.pins.gpio15)?;

    loop {
        led.toggle()?;
        FreeRtos::delay_ms(2000);
        for (addr, ds18b20) in &sensors {
            let resolution = ds18b20.measure_temperature(&mut wire, &mut delay).unwrap();
            delay.delay_ms(u32::from(resolution.time_ms()));
            if let Ok(temperature) = ds18b20
                .read_temperature(&mut wire, &mut delay)
                .map(ds18b20::split_temp)
                .map(|(integral, fractions)| f64::from(integral) + f64::from(fractions) / 10000.0)
            {
                info!("success! Temperature: {temperature}");
                LATEST_TEMPS
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                    .unwrap()
                    .insert(*addr, temperature);
            } else {
                warn!("unable to read temp");
            }
        }
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
