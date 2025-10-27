//! A ESP32C6 embedded solution for a pool temperature valve
//!
//! ## GPIO Pin Configuration
//! `OneWire` temperature sensor pins can be easily added/removed by modifying `ONEWIRE_PINS` array.
//! To add a third bus, you would:
//! 1. Update `ONEWIRE_PINS` (e.g., `&[2, 21, 4]`)
//! 2. Add a new type alias (e.g., `type OneWirePin3 = PinDriver<...>`)
//! 3. Update `initialize_external_temperature_sensors()` to handle the new pin
//! 4. Update `read_temperature_sensors()` to read from the new wire
//!
//! Current pin assignments:
//! - `GPIO2`: First `OneWire` temperature sensor (DS18B20)
//! - `GPIO21`: Second `OneWire` temperature sensor (DS18B20)  
//! - `GPIO1`: Relay control (output)
//! - `GPIO15`: Status LED (output)
//!
//! Note: ESP-IDF HAL requires different types for each GPIO pin, so adding buses
//! still requires some manual updates to the initialization and reading functions.

mod secrets;
mod webserver;

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use anyhow::anyhow;
use esp_idf_hal::{
    delay::Delay,
    gpio::PinDriver,
    sys::EspError,
    temp_sensor::{TempSensorConfig, TempSensorDriver},
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    log::{set_target_level, EspLogger},
    nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault},
    sys,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::{debug, info, warn};
use onewire::{ds18b20, DeviceSearch, OneWire, OpenDrainOutput, Sensor, DS18B20};

static LATEST_TEMPS: OnceLock<Mutex<HashMap<[u8; 8], f64>>> = OnceLock::new();
static RELAY_STATE: OnceLock<Mutex<bool>> = OnceLock::new();

// Special key for internal temperature sensor (all zeros to distinguish from real DS18B20 addresses)
const INTERNAL_TEMP_KEY: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

// GPIO pin configuration - change these constants to use different pins
// Note: When changing pins, you also need to update the pin initialization in main()
const ONEWIRE_PINS: &[u8] = &[2, 21]; // OneWire temperature sensor pins
                                      // To add a third bus: const ONEWIRE_PINS: &[u8] = &[2, 21, 4];
const RELAY_PIN: u8 = 1; // GPIO1 for relay control
                         // const LED_PIN: u8 = 15; // GPIO15 for status LED

// Type aliases for improved readability
// When adding pins, add corresponding type aliases here
type OneWirePin1 = PinDriver<'static, esp_idf_hal::gpio::Gpio2, esp_idf_hal::gpio::InputOutput>; // GPIO2
type OneWirePin2 = PinDriver<'static, esp_idf_hal::gpio::Gpio21, esp_idf_hal::gpio::InputOutput>; // GPIO21

type RelayPin = PinDriver<'static, esp_idf_hal::gpio::Gpio1, esp_idf_hal::gpio::Output>; // GPIO1 (RELAY_PIN)

// map from the 8 byte address of the sensor to the sensor itself
type SensorMap = HashMap<[u8; 8], DS18B20>;

// OneWire bus representation
struct OneWireBus {
    sensors: SensorMap,
    _pin_number: u8, // For identification and logging
}

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

    let mut wifi = initialize_wifi(peripherals.modem, sys_loop, nvs_partition)?;
    let ts_driver = initialize_temperature_sensor(peripherals.temp_sensor)?;
    let (onewire_buses, mut onewire_pin1, mut onewire_pin2) =
        initialize_external_temperature_sensors(peripherals.pins.gpio2, peripherals.pins.gpio21)?;
    let (mut relay_pin, mut last_relay_state) = initialize_relay(peripherals.pins.gpio1)?;

    info!("Setting up a web server");
    let mut server = webserver::create_server(&mut wifi)?;
    webserver::setup_http_handler(&mut server)?;

    info!("starting the main loop");
    let mut led = PinDriver::output(peripherals.pins.gpio15)?;
    let mut delay = esp_idf_hal::delay::Delay::new(10000);

    let mut wire1 = OneWire::new(&mut onewire_pin1, false);
    let mut wire2 = OneWire::new(&mut onewire_pin2, false);

    loop {
        led.toggle()?;
        delay.delay_ms(2000);

        // Read internal temperature and store it
        if let Ok(internal_temp) = ts_driver.get_fahrenheit() {
            LATEST_TEMPS
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .unwrap()
                .insert(INTERNAL_TEMP_KEY, f64::from(internal_temp));
        }

        read_temperature_sensors(&onewire_buses, &mut wire1, &mut wire2, &mut delay);
        update_relay_if_changed(&mut relay_pin, &mut last_relay_state)?;
    }
}

fn initialize_wifi(
    modem: esp_idf_hal::modem::Modem,
    sys_loop: EspSystemEventLoop,
    nvs_partition: EspDefaultNvsPartition,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    info!("Setting up wifi");
    let delay = esp_idf_hal::delay::Delay::new(10000);
    let espwifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs_partition))?;
    let mut wifi = BlockingWifi::wrap(espwifi, sys_loop)?;
    while let Err(e) = connect_wifi(&mut wifi) {
        warn!("Error connecting to wifi: {e}, retrying in 11s");
        delay.delay_ms(11_000);
    }
    Ok(wifi)
}

fn initialize_temperature_sensor(
    temp_sensor: esp_idf_hal::temp_sensor::TempSensor,
) -> anyhow::Result<TempSensorDriver<'static>> {
    info!("Setting up the temperature sensor");
    let mut ts_driver = TempSensorDriver::new(&TempSensorConfig::default(), temp_sensor)?;
    ts_driver.enable()?;
    info!("Temperature is {}F", ts_driver.get_fahrenheit()?);
    Ok(ts_driver)
}

fn initialize_external_temperature_sensors(
    onewire_gpio1: esp_idf_hal::gpio::Gpio2,
    onewire_gpio2: esp_idf_hal::gpio::Gpio21,
) -> anyhow::Result<(Vec<OneWireBus>, OneWirePin1, OneWirePin2)> {
    let mut delay = esp_idf_hal::delay::Delay::new(10000);
    let mut buses = Vec::new();

    info!(
        "Setting up the temperature probe on GPIO{}",
        ONEWIRE_PINS[0]
    );
    let mut onewire_pin1 = PinDriver::input_output_od(onewire_gpio1)?;
    let mut wire1 = OneWire::new(&mut onewire_pin1, false);
    let onewire1_sensors = find_devices(&mut wire1, &mut delay);
    buses.push(OneWireBus {
        sensors: onewire1_sensors,
        _pin_number: ONEWIRE_PINS[0],
    });

    info!(
        "Setting up the temperature probe on GPIO{}",
        ONEWIRE_PINS[1]
    );
    let mut onewire_pin2 = PinDriver::input_output_od(onewire_gpio2)?;
    let mut wire2 = OneWire::new(&mut onewire_pin2, false);
    let onewire2_sensors = find_devices(&mut wire2, &mut delay);
    buses.push(OneWireBus {
        sensors: onewire2_sensors,
        _pin_number: ONEWIRE_PINS[1],
    });

    Ok((buses, onewire_pin1, onewire_pin2))
}

fn initialize_relay(gpio1: esp_idf_hal::gpio::Gpio1) -> anyhow::Result<(RelayPin, bool)> {
    info!("Setting up the relay on GPIO{RELAY_PIN}");
    let mut relay_pin = PinDriver::output(gpio1)?;
    // get the value of the relay from the mutex and set it
    let last_relay_state = *RELAY_STATE
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
    Ok((relay_pin, last_relay_state))
}

fn read_temperature_sensors(
    onewire_buses: &[OneWireBus],
    wire1: &mut OneWire<&mut OneWirePin1>,
    wire2: &mut OneWire<&mut OneWirePin2>,
    delay: &mut Delay,
) {
    // Read temperatures from each OneWire bus
    // Note: This still assumes exactly 2 buses due to ESP-IDF HAL type constraints
    if let Some(bus) = onewire_buses.first() {
        read_sensors_from_wire(&bus.sensors, wire1, delay);
    }
    if let Some(bus) = onewire_buses.get(1) {
        read_sensors_from_wire(&bus.sensors, wire2, delay);
    }
}

fn read_sensors_from_wire<O: OpenDrainOutput>(
    sensors: &SensorMap,
    wire: &mut OneWire<O>,
    delay: &mut Delay,
) {
    for (addr, ds18b20) in sensors {
        debug!("reading temperature of {addr:?}");
        match get_temperature_f(ds18b20, wire, delay) {
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
}

fn update_relay_if_changed(
    relay_pin: &mut RelayPin,
    last_relay_state: &mut bool,
) -> anyhow::Result<()> {
    let relay_state = *RELAY_STATE.get().unwrap().lock().unwrap();
    if relay_state == *last_relay_state {
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
        *last_relay_state = relay_state;
    }
    Ok(())
}

fn find_devices<O: OpenDrainOutput>(wire: &mut OneWire<O>, delay: &mut Delay) -> SensorMap {
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
