use esp_idf_hal::{delay::FreeRtos, gpio::PinDriver, ledc::LedcDriver, prelude::*};
use esp_idf_svc::hal::{
    ledc::{config::TimerConfig, LedcTimerDriver},
    prelude::Peripherals,
};
use log::info;
use onewire::OneWire;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();

    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::new().frequency(25.kHz().into()),
    )
    .unwrap();

    let mut channel = LedcDriver::new(
        peripherals.ledc.channel0,
        timer_driver,
        peripherals.pins.gpio15,
    )
    .unwrap();

    info!("starting duty cycle loop");

    let max_duty = channel.get_max_duty();

    let mut pin = PinDriver::input_output_od(peripherals.pins.gpio10).unwrap();

    let mut wire = OneWire::new(&mut pin, false);

    let mut delay = esp_idf_hal::delay::Delay::default();

    if wire.reset(&mut delay).is_err() {
        info!("pullup failed for pin {:?}", pin.pin())
    };

    loop {
        for numerator in 0..=10 {
            info!("{numerator}");
            channel.set_duty(max_duty * numerator / 10).unwrap();
            FreeRtos::delay_ms(1000);
        }
    }
}
