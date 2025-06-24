// This module implements the 'Terrarium' interface for the actual esp32-based
// terrarium. It initializes and controls all of the hardware including the led
// driver, mist driver, fan driver, and sht30 temperature+humidity sensor.
//
// !WARNING!
//
// It is possible to fry the mist driver circuit by making changes to the code
// in this file. The mist driver circuit expects a 50% duty cycle ~108kHz
// signal from gpio4 of the esp32. If you instead turn gpio4 fully on, the mist
// driver mosfet will likely let out the magic smoke in a matter of several
// seconds. The duty cycle and frequency can be adjusted to different values,
// just don't turn gpio4 *fully on* for very long.
//
// !WARNING!

use esp_idf_hal::delay::Delay;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::i2c::*;
use esp_idf_hal::ledc::{LEDC, LedcDriver, LedcTimerDriver, config::TimerConfig};
use esp_idf_hal::sys::EspError;
use esp_idf_hal::temp_sensor::{TempSensor, TempSensorConfig, TempSensorDriver};
use esp_idf_hal::units::*;
use sht3x::{Address, ClockStretch, Repeatability, Sht3x};
use std::time::Duration;
use terralib::terrarium::Terrarium;
use terralib::types::SensorValues;

// value from 0 to 1 indicating the limit for led power. This is here to prevent
// the leds from being on too high and causing issues with overheating.
const LED_MAX: f32 = 0.8;

const MISTER_FREQ: KiloHertz = KiloHertz(108);

pub struct RealTerrarium<'a> {
    mist_channel: LedcDriver<'a>,
    led_channel: LedcDriver<'a>,
    // note: when using ledc fade features, get_duty() doesn't return the right thing, so we record it here.
    led_target: f32,
    fan_output: PinDriver<'a, gpio::Gpio3, gpio::Output>,
    sht30: Sht3x<I2cDriver<'a>>,
    // esp32 on-board temperature sensor.
    temp_sensor_driver: Option<TempSensorDriver<'a>>,
}

impl<'a> RealTerrarium<'a> {
    pub fn new(
        ledc: LEDC,
        sda: gpio::Gpio0,
        scl: gpio::Gpio1,
        mist_pin: gpio::Gpio4,
        fan_pin: gpio::Gpio3,
        led_pin: gpio::Gpio10,
        i2c: I2C0,
        temp_sensor: TempSensor,
    ) -> anyhow::Result<RealTerrarium<'a>, EspError> {
        // setup mister
        let mut mist_channel = LedcDriver::new(
            ledc.channel0,
            LedcTimerDriver::new(
                ledc.timer0,
                &TimerConfig::new().frequency(MISTER_FREQ.into()),
            )?,
            mist_pin,
        )?;
        mist_channel.set_duty(0)?;

        // setup led driver
        assert!(MISTER_FREQ.0 > 100 && MISTER_FREQ.0 < 150);
        let mut led_channel = LedcDriver::new(
            ledc.channel1,
            LedcTimerDriver::new(ledc.timer1, &TimerConfig::new().frequency(30.kHz().into()))?,
            led_pin,
        )?;
        led_channel.set_duty(0)?;

        // setup fans
        let mut fan_output = PinDriver::output(fan_pin)?;
        fan_output.set_low()?;

        // setup temp/humidity sensor
        let config = I2cConfig::new()
            .baudrate(400.kHz().into())
            .timeout(Duration::from_millis(1).into());
        let i2c = I2cDriver::new(i2c, sda, scl, &config)?;
        let mut sht30 = Sht3x::new(i2c, Address::Low);
        match sht30.status(&mut Delay::new(100)) {
            Ok(status) => log::info!("sht30 status raw: {status:?}"),
            Err(err) => log::error!("Error getting sht30 status: {:?}", err),
        };

        // Setup esp32 internal temp sensor.
        let temp_sens_cfg = TempSensorConfig::default();
        let temp_sensor_driver = match TempSensorDriver::new(&temp_sens_cfg, temp_sensor) {
            Ok(mut drv) => match drv.enable() {
                Ok(()) => Some(drv),
                Err(err) => {
                    log::error!("Failed to enable esp32 internal temp sensor: {err}");
                    None
                }
            },
            Err(err) => {
                log::error!("Failed to setup esp32 internal temp sensor: {err}");
                None
            }
        };

        let led_target = 0.0;
        Ok(Self {
            mist_channel,
            led_channel,
            led_target,
            fan_output,
            sht30,
            temp_sensor_driver,
        })
    }
}

impl<'a> Terrarium for RealTerrarium<'a> {
    fn set_lights(&mut self, val: f32) {
        self.set_lights_with_fade(val, 0);
    }

    // TODO: implement an async version of this that continues when the fade is done
    fn set_lights_with_fade(&mut self, val: f32, fade_ms: i32) {
        self.led_target = val;
        let duty = (LED_MAX * val * (self.led_channel.get_max_duty() as f32)) as u32;

        // TODO: consider changing the Terrarium trait to return errors
        if fade_ms > 0 {
            if let Err(err) = self.led_channel.fade_with_time(duty, fade_ms, false) {
                log::error!("Error fading leds: {err}");
            }
        } else if let Err(err) = self.led_channel.set_duty(duty) {
            log::error!("Error setting led duty cycle: {err}");
        }
    }

    fn get_lights(&self) -> f32 {
        self.led_target
    }

    fn set_mist(&mut self, on: bool) {
        let duty = if on {
            self.mist_channel.get_max_duty() / 2
        } else {
            0
        };

        // TODO: consider changing the Terrarium trait to return errors
        if let Err(err) = self.mist_channel.set_duty(duty) {
            log::error!("Error setting mister duty cycle: {err}");
        }
    }

    fn get_mist(&self) -> bool {
        self.mist_channel.get_duty() > 0
    }

    fn set_fans(&mut self, on: bool) {
        let level = gpio::Level::from(on);
        // TODO: handle errors?
        if let Err(err) = self.fan_output.set_level(level) {
            log::error!("Error setting fan level: {err}");
        }
    }

    fn get_fans(&self) -> bool {
        self.fan_output.is_set_high()
    }

    fn read_sensors(&mut self) -> Option<SensorValues> {
        let m_result = self.sht30.measure(
            ClockStretch::Enabled,
            Repeatability::High,
            &mut Delay::new(0),
        );
        match m_result {
            // note: the sht3x library returns integer measurements 100x the
            // actual values, so we divide by 100 to get actual degrees celsius
            // and relative humidity %.
            Ok(m) => Some(SensorValues {
                temp: (m.temperature as f32) / 100.0,
                humid: (m.humidity as f32) / 100.0 / 100.0,
            }),
            Err(err) => {
                log::error!("Error reading sht30 sensor {:?}", err);
                None
            }
        }
    }

    fn read_cpu_temp(&mut self) -> Option<f32> {
        // esp32 internal temp sensor
        if let Some(sens) = &mut self.temp_sensor_driver {
            match sens.get_celsius() {
                Ok(t) => Some(t),
                Err(err) => {
                    log::error!("Error reading esp32 internal temperature: {}", err);
                    None
                }
            }
        } else {
            log::error!(
                "Esp32 internal temp sensor driver not installed. Something must have failed at startup."
            );
            None
        }
    }
}
