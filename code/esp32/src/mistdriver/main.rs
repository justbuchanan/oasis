//! Test program for driving an ultrasonic mister at 108kHz via pwm on gpio 4.

use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver, config::TimerConfig};
use esp_idf_hal::prelude::*;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let mut channel = LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &TimerConfig::new().frequency(110.kHz().into()),
        )?,
        // io4 is mist. io10 is led driver. io3 is fans.
        peripherals.pins.gpio4,
    )?;

    log::info!("Starting 108kHz pwm on gpio4");

    let max_duty = channel.get_max_duty();
    channel.set_duty(max_duty / 2)?; // 50% duty for mister

    // // turn it off
    // channel.set_duty(0)?;

    loop {
        thread::sleep(Duration::from_millis(100));
    }
}
