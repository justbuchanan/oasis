use esp_idf_hal::prelude::*;
use std::time::Duration;
use terralib::terrarium::{Terrarium, print_terrarium_info};
use terrarium::real_terrarium::RealTerrarium;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Initialize logging to stdout.
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hardware test starting...");

    let peripherals = Peripherals::take()?;

    let mut terrarium = RealTerrarium::new(
        peripherals.ledc,
        peripherals.pins.gpio0,
        peripherals.pins.gpio1,
        peripherals.pins.gpio4,
        peripherals.pins.gpio3,
        peripherals.pins.gpio10,
        peripherals.i2c0,
        peripherals.temp_sensor,
    )?;
    print_terrarium_info(&mut terrarium);

    loop {
        terrarium.set_lights(0.5);
        std::thread::sleep(Duration::from_secs(2));
        terrarium.set_lights(1.0);
        std::thread::sleep(Duration::from_secs(2));
        terrarium.set_lights(0.0);
        std::thread::sleep(Duration::from_secs(2));

        terrarium.set_fans(true);
        std::thread::sleep(Duration::from_secs(2));
        terrarium.set_fans(false);
        std::thread::sleep(Duration::from_secs(2));

        terrarium.set_mist(true);
        std::thread::sleep(Duration::from_secs(2));
        terrarium.set_mist(false);
        std::thread::sleep(Duration::from_secs(2));
    }
}
