//! Test program for communicating with the sht30 temp/humid sensor over i2c

use esp_idf_hal::delay::Delay;
use esp_idf_hal::i2c::*;
use esp_idf_hal::prelude::*;
use sht3x::{Address, ClockStretch, Repeatability, Sht3x};

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;

    let config = I2cConfig::new().baudrate(800.kHz().into());

    let sda = peripherals.pins.gpio0;
    let scl = peripherals.pins.gpio1;
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;
    let mut sht30 = Sht3x::new(i2c, Address::Low);
    println!(
        "Status raw: {:?}",
        sht30.status(&mut Delay::new(100)).unwrap()
    );

    loop {
        let m = sht30
            .measure(
                ClockStretch::Disabled,
                Repeatability::High,
                &mut Delay::new(100),
            )
            .unwrap();
        // note: the sht3x library returns integer measurements 100x the actual
        // values, so divide by 100 to get reasonable numbers.
        let t = (m.temperature as f32) / 100.0;
        let h = (m.humidity as f32) / 100.0;
        println!("Temp: {t:.2}C Humidity: {h:.2}");
    }
}
