use crate::config::{Schedule, TerrariumConfig, TerrariumConfigUpdate, Update};
use crate::terrarium::Terrarium;
use crate::types::{Actuator, ActuatorOverrideSet, ActuatorValue, ActuatorValues};
use anyhow::anyhow;
use embassy_time::Timer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Max amount of time that a control override can specify is 30 minutes.
const MAX_OVERRIDE_DURATION_SECS: u32 = 30 * 60;

const DEFAULT_TIMEZONE: &'static str = "America/Los_Angeles";

struct ActuatorOverride {
    value: ActuatorValue,
    start: Instant,
    duration: Duration,
}

// The TerrariumController manages the terrarium hardware and its configuration
// (schedule). It handles executing the schedule and managing overrides
// (temporary controls). Note that although wifi details are part of
// TerrariumConfig, wifi management is handled external to the controller.
pub struct TerrariumController {
    terrarium: Arc<Mutex<dyn Terrarium + Send>>,
    config: TerrariumConfig,
    active_overrides: HashMap<Actuator, ActuatorOverride>,
    // TODO: use a mutex for external_light_control?
    external_light_control: bool,
}

impl TerrariumController {
    pub fn new(terrarium: Arc<Mutex<dyn Terrarium + Send>>, config: TerrariumConfig) -> Self {
        Self {
            terrarium,
            config,
            active_overrides: HashMap::new(),
            external_light_control: false,
        }
    }

    // Tell the controller not to touch the lights until release_lights() is
    // later called. This is used by the hard reset functionality to take
    // control of the lights.
    pub fn takeover_lights(&mut self) {
        if self.external_light_control {
            panic!("Lights are already externally-controlled");
        }
        self.external_light_control = true;
    }

    pub fn release_lights(&mut self) {
        self.external_light_control = false;
    }

    pub fn terrarium(&self) -> Arc<Mutex<dyn Terrarium>> {
        self.terrarium.clone()
    }

    pub fn config(&self) -> &TerrariumConfig {
        &self.config
    }

    pub fn update_config(&mut self, update: &TerrariumConfigUpdate) -> anyhow::Result<()> {
        // validate updates first - we don't want to fail halfway through the
        // update and end up with an inconsistent state. If the update is bad,
        // fail early.
        update.validate()?;

        match &update.name {
            Update::Set(name) => self.config.name = Some(name.clone()),
            Update::Clear => self.config.name = None,
            Update::NoChange => {}
        };

        match &update.wifi {
            Update::Set(wifi) => self.config.wifi = Some(wifi.clone()),
            Update::Clear => self.config.wifi = None,
            Update::NoChange => {}
        };

        match &update.schedule {
            Update::Set(schedule_update) => {
                if self.config.schedule.is_none() {
                    self.config.schedule = Some(Schedule::default());
                }
                if let Some(schedule) = &mut self.config.schedule {
                    schedule.update(schedule_update);
                }
            }
            Update::Clear => self.config.schedule = None,
            Update::NoChange => {}
        };

        match &update.timezone {
            Update::Set(timezone) => self.config.timezone = Some(timezone.clone()),
            Update::Clear => self.config.timezone = None,
            Update::NoChange => {}
        }

        match &update.influxdb {
            Update::Set(influxdb) => self.config.influxdb = Some(influxdb.clone()),
            Update::Clear => self.config.influxdb = None,
            Update::NoChange => {}
        };

        Ok(())
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let t = self.get_local_time();

        let mut act_val = ActuatorValues::default();

        if let Some(schedule) = &self.config.schedule {
            // Turn on actuators based on the configured schedule.
            act_val = schedule.evaluate(t);

            // Automatic misting based on humidity_setpoint
            //
            // TODO: this implementation is very simplistic and could be improved a lot
            // - humidity readings lag partially due to the placement of the sensor and lack of air movement.
            // - periodically turning on the fans could help the sensor to get an accurate reading more often
            // - we should add some rate-limiting i.e. "mist for a maximum of one minute straight every ten minutes"
            if schedule.auto_mist_enabled {
                if let Some(setpoint) = schedule.humidity_setpoint {
                    match self.terrarium.lock().unwrap().read_sensors() {
                        Some(sensor_values) => {
                            if sensor_values.humid < setpoint {
                                act_val.mist = true; // Turn mist ON if below setpoint
                            }
                        }
                        None => {
                            log::warn!("Failed to read sensors for auto-mist control.");
                        }
                    }
                }
            }
        }

        let instant_now = Instant::now();

        // Apply overrides / temporary controls.
        // If an override is expired, remove it. Otherwise, use its value to
        // override whatever is configured in the schedule.
        if let Some(lights_override) = self.active_overrides.get(&Actuator::Lights) {
            if instant_now - lights_override.start > lights_override.duration {
                self.active_overrides.remove(&Actuator::Lights);
            } else if let ActuatorValue::Float(l) = lights_override.value {
                act_val.lights = l;
            } else {
                // PANIC - this shouldn't happen
            }
        }
        if let Some(mist_override) = self.active_overrides.get(&Actuator::Mist) {
            if instant_now - mist_override.start > mist_override.duration {
                self.active_overrides.remove(&Actuator::Mist);
            } else if let ActuatorValue::Bool(m) = mist_override.value {
                act_val.mist = m;
            } else {
                // PANIC - this shouldn't happen
            }
        }
        if let Some(fan_override) = self.active_overrides.get(&Actuator::Fans) {
            if instant_now - fan_override.start > fan_override.duration {
                self.active_overrides.remove(&Actuator::Fans);
            } else if let ActuatorValue::Bool(f) = fan_override.value {
                act_val.fans = f;
            } else {
                // PANIC - this shouldn't happen
            }
        }

        let mut terrarium = self.terrarium.lock().unwrap();
        if !self.external_light_control && act_val.lights != terrarium.get_lights() {
            terrarium.set_lights_with_fade(act_val.lights, 100);
        }
        terrarium.set_mist(act_val.mist);
        terrarium.set_fans(act_val.fans);

        Ok(())
    }

    // A control command specifies overrides to apply to the lights, fans,
    // and/or mister. This function adds ActuatorOverride entries to the
    // controller's active_overrides list, but they are not actually executed
    // until the next call to run().
    pub fn handle_control_cmd(&mut self, update_data: &ActuatorOverrideSet) -> anyhow::Result<()> {
        if update_data.updates.is_empty() {
            return Err(anyhow!("Empty control request"));
        }

        for ud in &update_data.updates {
            match ud.actuator {
                Actuator::Mist => match ud.value {
                    ActuatorValue::Bool(_) => {
                        let duration = std::cmp::min(ud.duration_secs, MAX_OVERRIDE_DURATION_SECS);
                        self.active_overrides.insert(
                            Actuator::Mist,
                            ActuatorOverride {
                                value: ud.value,
                                duration: Duration::from_secs(duration as u64),
                                start: Instant::now(),
                            },
                        );
                    }
                    _ => return Err(anyhow!("Expected bool for mist")),
                },
                Actuator::Lights => match ud.value {
                    ActuatorValue::Float(l) => {
                        let duration = std::cmp::min(ud.duration_secs, MAX_OVERRIDE_DURATION_SECS);
                        if !(0.0..=1.0).contains(&l) {
                            return Err(anyhow!(
                                "Lights value should be in the between 0 and 1, got {}",
                                l
                            ));
                        }
                        self.active_overrides.insert(
                            Actuator::Lights,
                            ActuatorOverride {
                                value: ud.value,
                                duration: Duration::from_secs(duration as u64),
                                start: Instant::now(),
                            },
                        );
                    }
                    _ => return Err(anyhow!("Expected float for lights")),
                },
                Actuator::Fans => match ud.value {
                    ActuatorValue::Bool(_) => {
                        let duration = std::cmp::min(ud.duration_secs, MAX_OVERRIDE_DURATION_SECS);
                        self.active_overrides.insert(
                            Actuator::Fans,
                            ActuatorOverride {
                                value: ud.value,
                                duration: Duration::from_secs(duration as u64),
                                start: Instant::now(),
                            },
                        );
                    }
                    _ => return Err(anyhow!("Expected bool for fan")),
                },
            }
        }

        Ok(())
    }

    // Uses the configured timezone if possible, otherwise defaults to US West Coast time.
    fn get_local_time(&self) -> jiff::civil::Time {
        jiff::Timestamp::now().to_zoned(self.get_timezone()).time()
    }

    fn get_timezone(&self) -> jiff::tz::TimeZone {
        self.config
            .timezone
            .as_ref()
            .and_then(|tz_name| {
                jiff::tz::TimeZone::get(tz_name)
                    .map_err(|err| {
                        log::warn!("Invalid timezone: '{}', {}", tz_name, err);
                        err
                    })
                    .ok()
            })
            .unwrap_or_else(|| {
                jiff::tz::TimeZone::get(DEFAULT_TIMEZONE)
                    .expect("Default timezone should always be valid")
            })
    }
}

// spins until it can get the mutex. If Mutex.lock() is used directly in async code, we can (and do) get deadlocks.
// TODO: there has to be a better way to do this. Look into NoopRawMutex from embassy.
pub async fn spin_lock_mutex<T: ?Sized>(
    mutex: &std::sync::Mutex<T>,
) -> std::sync::MutexGuard<'_, T> {
    loop {
        if let Ok(m) = mutex.try_lock() {
            return m;
        }
        Timer::after(embassy_time::Duration::from_millis(5)).await;
    }
}

#[embassy_executor::task]
pub async fn terrarium_controller_main_loop(controller: Arc<Mutex<TerrariumController>>) {
    loop {
        Timer::after(embassy_time::Duration::from_millis(1)).await;

        if let Err(err) = spin_lock_mutex(&*controller).await.run() {
            log::error!("Terrarium run() errored with: {err}");
        }
    }
}

#[cfg(test)]
mod controller {
    use super::*;
    use crate::config::WifiDetails;
    use crate::terrarium::FakeTerrarium;
    use crate::types::ActuatorOverride;

    #[test]
    fn update_config() {
        let mut cfg = TerrariumConfig::default();
        cfg.name = Some("foo".to_string());
        cfg.wifi = Some(WifiDetails {
            ssid: "ssid1".to_string(),
            password: "password1".to_string(),
        });
        let mut ctl = TerrariumController::new(Arc::new(Mutex::new(FakeTerrarium::new())), cfg);

        let mut cfg_update = TerrariumConfigUpdate::default();
        cfg_update.name = Update::Set("bar".to_string());
        assert!(ctl.update_config(&cfg_update).is_ok());

        assert_eq!(
            ctl.config.wifi,
            Some(WifiDetails {
                ssid: "ssid1".to_string(),
                password: "password1".to_string(),
            })
        );
        assert_eq!(ctl.config.name, Some("bar".to_string()));
    }

    #[test]
    fn basic() {
        let mut ctl = TerrariumController::new(
            Arc::new(Mutex::new(FakeTerrarium::new())),
            TerrariumConfig::default(),
        );
        assert!(!ctl.terrarium().lock().unwrap().get_mist());
        assert!(!ctl.terrarium().lock().unwrap().get_fans());
        assert_eq!(ctl.terrarium().lock().unwrap().get_lights(), 0.0);
        let ud = ActuatorOverrideSet {
            updates: vec![
                ActuatorOverride {
                    actuator: Actuator::Mist,
                    value: ActuatorValue::Bool(true),
                    duration_secs: 5,
                },
                ActuatorOverride {
                    actuator: Actuator::Fans,
                    value: ActuatorValue::Bool(true),
                    duration_secs: 10,
                },
                ActuatorOverride {
                    actuator: Actuator::Lights,
                    value: ActuatorValue::Float(0.7),
                    duration_secs: 15,
                },
            ],
        };
        assert!(ctl.handle_control_cmd(&ud).is_ok());
        assert!(ctl.run().is_ok());
        assert!(ctl.terrarium().lock().unwrap().get_mist());
        assert!(ctl.terrarium().lock().unwrap().get_fans());
        assert_eq!(ctl.terrarium().lock().unwrap().get_lights(), 0.7);
    }

    #[test]
    fn invalid_type() {
        let mut ctl = TerrariumController::new(
            Arc::new(Mutex::new(FakeTerrarium::new())),
            TerrariumConfig::default(),
        );
        let ud = ActuatorOverrideSet {
            updates: vec![ActuatorOverride {
                actuator: Actuator::Mist,
                value: ActuatorValue::Float(1000.0),
                duration_secs: 100,
            }],
        };
        let result = ctl.handle_control_cmd(&ud);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Expected bool for mist",);
    }

    #[test]
    fn test_auto_mist() {
        let terrarium = Arc::new(Mutex::new(FakeTerrarium::new()));
        let cfg = TerrariumConfig {
            schedule: Some(Schedule {
                auto_mist_enabled: true,
                humidity_setpoint: Some(0.8),
                ..Schedule::default()
            }),
            ..TerrariumConfig::default()
        };
        let mut ctl = TerrariumController::new(terrarium.clone(), cfg);
        // start at humidity 0.5
        terrarium
            .lock()
            .unwrap()
            .state
            .sensors
            .as_mut()
            .unwrap()
            .humid = 0.5;
        ctl.run().unwrap();
        assert!(
            terrarium.lock().unwrap().get_mist(),
            "Mist should be on when humidity is low"
        );

        terrarium
            .lock()
            .unwrap()
            .state
            .sensors
            .as_mut()
            .unwrap()
            .humid = 0.81;
        ctl.run().unwrap();
        assert!(
            !terrarium.lock().unwrap().get_mist(),
            "Mist should be off when humidity is high"
        );
    }
}

#[cfg(test)]
mod json_format {
    use super::*;
    use crate::types::ActuatorOverride;

    #[test]
    fn serialize() {
        let data = ActuatorOverrideSet {
            updates: vec![
                ActuatorOverride {
                    actuator: Actuator::Mist,
                    value: ActuatorValue::Bool(true),
                    duration_secs: 10,
                },
                ActuatorOverride {
                    actuator: Actuator::Lights,
                    value: ActuatorValue::Float(0.5),
                    duration_secs: 15,
                },
            ],
        };

        assert_eq!(
            serde_json::to_string(&data).unwrap(),
            "{\"updates\":[{\"actuator\":\"mist\",\"value\":true,\"duration_secs\":10},{\"actuator\":\"lights\",\"value\":0.5,\"duration_secs\":15}]}".to_string()
        );
    }
}
