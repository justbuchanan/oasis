//! Simple test program that blinks an led on gpio 4.

use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, worlds!");

    let peripherals = Peripherals::take()?;
    let mut led = PinDriver::output(peripherals.pins.gpio4)?;

    loop {
        led.set_high()?;
        // we are sleeping here to make sure the watchdog isn't triggered
        std::thread::sleep(Duration::from_millis(1000));

        led.set_low()?;
        std::thread::sleep(Duration::from_millis(1000));
    }
}
