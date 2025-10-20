//! A ESP32C6 embedded solution for a pool temperature valve

mod secrets;

use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::{Mutex, OnceLock},
};

use anyhow::anyhow;
use esp_idf_hal::{
    delay::Delay,
    gpio::PinDriver,
    io::Write as _,
    sys::EspError,
    temp_sensor::{TempSensorConfig, TempSensorDriver},
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    http::{self, server::EspHttpServer, Method},
    log::{set_target_level, EspLogger},
    nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault},
    sys,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::{debug, info, warn};
use onewire::{ds18b20, DeviceSearch, OneWire, OpenDrainOutput, Sensor, DS18B20};

static LATEST_TEMPS: OnceLock<Mutex<HashMap<[u8; 8], f64>>> = OnceLock::new();
static RELAY_STATE: OnceLock<Mutex<bool>> = OnceLock::new();

#[derive(Debug)]
struct Preferences {
    hysteresis: u16,
    min_on_temp_f: u8,
}

impl Preferences {
    const DEFAULT_HYSTERESIS: u16 = 2;
    const DEFAULT_MIN_ON_TEMP_F: u8 = 70;

    fn from_nvs(nvs: &EspNvs<NvsDefault>) -> Result<Self, EspError> {
        Ok(Self {
            hysteresis: nvs
                .get_u16("hysteresis")?
                .unwrap_or(Self::DEFAULT_HYSTERESIS),
            min_on_temp_f: nvs
                .get_u8("min_on_temp_f")?
                .unwrap_or(Self::DEFAULT_MIN_ON_TEMP_F),
        })
    }

    #[expect(unused)]
    fn save_to_nvs(&self, nvs: &EspNvs<NvsDefault>) -> Result<(), EspError> {
        nvs.set_u16("hysteresis", self.hysteresis)?;
        nvs.set_u8("min_on_temp_f", self.min_on_temp_f)?;
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly.
    // See https://github.com/esp-rs/esp-idf-template/issues/71
    sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    // only produce global warnings, but info for our own stuff
    EspLogger::initialize_default();
    set_target_level("*", log::LevelFilter::Warn)?;
    set_target_level("erik", log::LevelFilter::Info)?;

    info!("Fetching resources from hardware");
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs_partition = EspDefaultNvsPartition::take()?;
    let nvs = EspNvs::new(nvs_partition.clone(), "pool", true)?;

    // TODO: make preferences mutable and allow them to change from the web
    // this probably means adding locks and making preferences more global
    let preferences = Preferences::from_nvs(&nvs)?;
    info!("{preferences:?}");

    info!("Setting up wifi");
    let mut delay = esp_idf_hal::delay::Delay::new(10000);
    let espwifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs_partition))?;
    let mut wifi = BlockingWifi::wrap(espwifi, sys_loop)?;
    while let Err(e) = connect_wifi(&mut wifi) {
        warn!("Error connecting to wifi: {e}, retrying in 11s");
        delay.delay_ms(11_000);
    }

    info!("Setting up the temperature sensor");
    let mut ts_driver =
        TempSensorDriver::new(&TempSensorConfig::default(), peripherals.temp_sensor)?;
    ts_driver.enable()?;
    info!("Temperature is {}F", ts_driver.get_fahrenheit()?);

    info!("Setting up the temperature probe on Gpio2");
    let mut pin = PinDriver::input_output_od(peripherals.pins.gpio2)?;
    let mut wire = OneWire::new(&mut pin, false);
    let sensors = find_devices(&mut wire, &mut delay);

    info!("setting up the temperature probe on gpio21");
    let mut pin = PinDriver::input_output_od(peripherals.pins.gpio21)?;
    let mut wire = OneWire::new(&mut pin, false);
    let sensors2 = find_devices(&mut wire, &mut delay);

    info!("Setting up the relay on gpio1");
    let mut relay_pin = PinDriver::output(peripherals.pins.gpio1)?;
    // get the value of the relay from the mutex and set it
    let mut last_relay_state = *RELAY_STATE
        .get_or_init(|| Mutex::new(false))
        .lock()
        .unwrap();
    if last_relay_state {
        info!("Setting relay to ON");
        relay_pin.set_high()?;
    } else {
        info!("Setting relay to OFF");
        relay_pin.set_low()?;
    }
    info!("Setting up a web server");
    let mut server = create_server(&mut wifi)?;
    server.fn_handler("/", Method::Get, move |req| {
        // Extract the query string from the URI
        let uri = req.uri();
        let relay_param = uri.split_once('?').map(|x| x.1).and_then(|qs| {
            qs.split('&').find_map(|kv| {
                let mut parts = kv.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some("relay"), Some(value)) => Some(value),
                    _ => None,
                }
            })
        });

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
                    temp,
                )
            })
            .fold(String::new(), |mut output, (addr, temp): (String, f64)| {
                // leading comma is safe here because we always have an internal
                // temperature first
                let _ = write!(output, r#", "{addr}": {temp}"#);
                output
            });

        // fetch the relay state from the mutex
        let relay_state = *RELAY_STATE
            .get()
            .expect("lock was crated earlier")
            .lock()
            .unwrap();

        req.into_ok_response()?.write_all(
            format!(
                r#"{{
                        "units": "farenheit",
                        "relay": {relay_state},
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

    info!("starting the main loop");

    let mut led = PinDriver::output(peripherals.pins.gpio15)?;

    loop {
        led.toggle()?;
        delay.delay_ms(2000);
        for (addr, ds18b20) in &sensors {
            debug!("reading temperature of {addr:?}");
            match get_temperature_f(ds18b20, &mut wire, &mut delay) {
                Ok(temperature) => {
                    info!("success! Temperature: {temperature}");
                    LATEST_TEMPS
                        .get_or_init(|| Mutex::new(HashMap::new()))
                        .lock()
                        .unwrap()
                        .insert(*addr, temperature);
                }
                Err(e) => {
                    warn!("unable to read temp: {e:?}");
                }
            }
        }
        for (addr, ds18b20) in &sensors2 {
            debug!("reading temperature of {addr:?}");
            match get_temperature_f(ds18b20, &mut wire, &mut delay) {
                Ok(temperature) => {
                    info!("success! Temperature: {temperature}");
                    LATEST_TEMPS
                        .get_or_init(|| Mutex::new(HashMap::new()))
                        .lock()
                        .unwrap()
                        .insert(*addr, temperature);
                }
                Err(e) => {
                    warn!("unable to read temp: {e:?}");
                }
            }
        }

        // check for changes in the relay state
        let relay_state = *RELAY_STATE.get().unwrap().lock().unwrap();
        if relay_state == last_relay_state {
            debug!(
                "Relay state unchanged: {}",
                if relay_state { "ON" } else { "OFF" }
            );
        } else {
            info!(
                "Relay state changed to {}",
                if relay_state { "ON" } else { "OFF" }
            );

            if relay_state {
                relay_pin.set_high()?;
            } else {
                relay_pin.set_low()?;
            }
            last_relay_state = relay_state;
        }
    }

    /*
    I think we want to implement our own loop, so we never
    exit from main, but we could have a service handler that
    reads the temperatures periodically instead, then we'd
    just need to keep the wifi and server objects around...
    forget(wifi);
    forget(server);
    Ok(()) */
}

fn find_devices<O: OpenDrainOutput>(
    wire: &mut OneWire<O>,
    delay: &mut Delay,
) -> HashMap<[u8; 8], DS18B20> {
    let mut sensors = HashMap::new();
    if wire.reset(delay).is_err() {
        warn!("extern temperature probe reset failed");
    } else {
        let mut search = DeviceSearch::new();
        while let Ok(Some(device)) = wire.search_next(&mut search, delay) {
            info!("found device {device:?}");
            let addr = device.address;
            if let Ok(ds18b20) = DS18B20::new(device) {
                info!("and it was a ds18b20");
                if let Ok(temperature) = get_temperature_f(&ds18b20, wire, delay) {
                    info!("success! Temperature: {temperature}");

                    LATEST_TEMPS
                        .get_or_init(|| Mutex::new(HashMap::new()))
                        .lock()
                        .unwrap()
                        .insert(addr, temperature);
                } else {
                    warn!("Sensor at {addr:?} didn't get a temperature");
                }
                // insert it even if we didn't get a temperature
                sensors.insert(addr, ds18b20);
            } else {
                warn!("Device didn't seem to be a ds18b20");
            }
        }
    }
    sensors
}

fn get_temperature_f<O: OpenDrainOutput>(
    ds18b20: &DS18B20,
    wire: &mut OneWire<O>,
    delay: &mut Delay,
) -> Result<f64, onewire::Error<O::Error>> {
    let delaytime = ds18b20.start_measurement(wire, delay).unwrap();
    delay.delay_ms(u32::from(delaytime) + 50);
    ds18b20
        .read_temperature(wire, delay)
        .map(ds18b20::split_temp)
        .map(|(integral, fractions)| f64::from(integral) + f64::from(fractions) / 10000.0)
        .map(|temp| temp * 9. / 5. + 32.)
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
