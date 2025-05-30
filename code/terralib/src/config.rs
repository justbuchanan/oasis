use crate::influxdb;
use crate::types::ActuatorValues;
use anyhow::anyhow;
use jiff::civil::Time;
use serde;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct TerrariumConfig {
    pub name: Option<String>,
    pub wifi: Option<WifiDetails>,
    pub schedule: Option<Schedule>,
    pub influxdb: Option<influxdb::Config>,
}

impl TerrariumConfig {
    pub fn new_with_reasonable_defaults() -> Self {
        Self {
            name: Some("oasis".into()),
            wifi: None,
            schedule: Some(Schedule::new_with_reasonable_defaults()),
            influxdb: None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct WifiDetails {
    pub ssid: String,
    pub password: String,
}

// TODO: unify this with ScheduledEvent?
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TimeRange {
    pub start: Time,
    pub stop: Time,
}

// Schedule describes when the lights, fans, and mister should turn on/off
// throughout the day.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct Schedule {
    pub lights: Option<TimeRange>,
    pub light_intensity: Option<f32>,
    pub fans: Vec<ScheduledEvent>,
    // misting schedule. only relevant when mist_mode==Manual.
    pub mist: Vec<ScheduledEvent>,
    pub mist_mode: HumidityMode,
    // target minimum humidity. only relevant when mist_mode==Auto.
    pub humidity_setpoint: Option<f32>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct ScheduledEvent {
    pub start_time: Time,
    pub duration_secs: u32,
    pub repeat: Option<RepeatInfo>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct RepeatInfo {
    pub n_hours: u32,
    pub stop_time: Time,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Copy, Default)]
pub enum HumidityMode {
    Auto,
    #[default]
    Manual,
}

impl Schedule {
    // The "reasonable defaults" schedule should be set so that if the user
    // never changes it, their plants will do ok.
    pub fn new_with_reasonable_defaults() -> Self {
        Self {
            lights: Some(TimeRange {
                start: "10:00".parse().unwrap(),
                stop: "22:00".parse().unwrap(),
            }),
            light_intensity: Some(0.6),
            fans: vec![ScheduledEvent {
                start_time: "10:30".parse().unwrap(),
                duration_secs: 10 * 60,
                repeat: Some(RepeatInfo {
                    n_hours: 1,
                    stop_time: "22:00".parse().unwrap(),
                }),
            }],
            mist: vec![ScheduledEvent {
                start_time: "10:01".parse().unwrap(),
                duration_secs: 30,
                repeat: None,
            }],
            humidity_setpoint: None,
            mist_mode: HumidityMode::Manual,
        }
    }

    pub fn update(&mut self, update: &ScheduleUpdate) {
        match &update.lights {
            Update::Set(lights) => self.lights = Some(lights.clone()),
            Update::Clear => self.lights = None,
            Update::NoChange => {}
        }

        match &update.light_intensity {
            Update::Set(intensity) => self.light_intensity = Some(*intensity),
            Update::Clear => self.light_intensity = None,
            Update::NoChange => {}
        }

        match &update.fans {
            Update::Set(fans) => self.fans = fans.clone(),
            Update::Clear => self.fans = vec![],
            Update::NoChange => {}
        }

        match &update.mist {
            Update::Set(mist) => self.mist = mist.clone(),
            Update::Clear => self.mist = vec![],
            Update::NoChange => {}
        }

        match update.mist_mode {
            Update::Set(mist_mode) => self.mist_mode = mist_mode,
            Update::Clear => self.mist_mode = HumidityMode::default(),
            Update::NoChange => {}
        }

        match update.humidity_setpoint {
            Update::Set(humidity_setpoint) => self.humidity_setpoint = Some(humidity_setpoint),
            Update::Clear => self.humidity_setpoint = None,
            Update::NoChange => {}
        }
    }

    pub fn evaluate(&self, t: Time) -> ActuatorValues {
        let mut v = ActuatorValues::default();

        if let Some(lights) = &self.lights {
            if let Some(intensity) = self.light_intensity {
                if lights.start < t && t < lights.stop {
                    v.lights = intensity;
                }
            }
        }

        if self.mist_mode == HumidityMode::Auto {
            // Auto mode is handled by the controller based on sensor readings.
            v.mist = false;
        } else {
            v.mist = evaluate_scheduled_events(&self.mist, t);
        }

        v.fans = evaluate_scheduled_events(&self.fans, t);

        v
    }
}

fn evaluate_scheduled_events(events: &Vec<ScheduledEvent>, t: Time) -> bool {
    for event in events {
        let mut start_time = event.start_time;
        let event_duration = std::time::Duration::from_secs(event.duration_secs.into());
        let mut end_time = start_time + event_duration;
        if start_time <= t && t <= end_time {
            return true;
        }

        if let Some(repeat) = &event.repeat {
            loop {
                start_time += std::time::Duration::from_hours(repeat.n_hours.into());
                if start_time > repeat.stop_time {
                    break;
                }
                end_time = start_time + event_duration;

                if start_time <= t && t <= end_time {
                    return true;
                }
            }
        }
    }

    false
}

// This type is very similar to `TerrariumConfig`, but is used for specifying
// updates. The problem being solved here is that when making a change to the
// config, we want to be able to do one of three things:
//
// - set the field to a new value
// - set the field to None
// - leave the field alone
//
// The Option type can express at most two of these situations, so we declare an
// `Update` type that can handle all three.
//
// serde is configured such that json is deserialized with these rules:
// - json field has a value -> Update::Set
// - json field is null -> Update::Clear
// - json field is not present -> Update::NoChange
#[derive(Default, Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TerrariumConfigUpdate {
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub name: Update<String>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub wifi: Update<WifiDetails>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub schedule: Update<ScheduleUpdate>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub influxdb: Update<influxdb::Config>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[derive(Default)]
pub enum Update<T> {
    #[serde(rename = "value")]
    Set(T),
    #[serde(rename = "null")]
    Clear,
    #[serde(skip)]
    #[default]
    NoChange,
}

impl<T> Update<T> {
    fn is_no_change(&self) -> bool {
        matches!(self, Update::NoChange)
    }
}

// See comment for TerrariumConfigUpdate.
#[derive(Default, Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ScheduleUpdate {
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub lights: Update<TimeRange>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub light_intensity: Update<f32>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub fans: Update<Vec<ScheduledEvent>>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub mist: Update<Vec<ScheduledEvent>>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub mist_mode: Update<HumidityMode>,
    #[serde(default, skip_serializing_if = "Update::is_no_change")]
    pub humidity_setpoint: Update<f32>,
}

impl TerrariumConfigUpdate {
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Update::Set(schedule_update) = &self.schedule {
            schedule_update.validate()?;
        }
        if let Update::Set(name) = &self.name {
            if name.len() > 30 {
                return Err(anyhow!("Name too long"));
            }
            if name.len() == 0 {
                return Err(anyhow!("Name can not be empty string"));
            }
            // TODO: name should be a valid domain name identifier
        }
        Ok(())
    }
}

impl ScheduleUpdate {
    // TODO: can we do validation at deserialization time instead? It'd be nice
    // if it was impossible to build an invalid ScheduleUpdate in the first
    // place.
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Update::Set(mist) = &self.mist {
            validate_scheduled_events(mist)?;
        }
        if let Update::Set(fans) = &self.fans {
            validate_scheduled_events(fans)?;
        }

        if let Update::Set(lights) = &self.lights {
            if lights.stop <= lights.start {
                return Err(anyhow!("light start time should be before stop time"));
            }
        }

        if let Update::Set(light_intensity) = self.light_intensity {
            if !(0.0..=1.0).contains(&light_intensity) {
                return Err(anyhow!(
                    "light_intensity must be between 0.0 and 1.0, got {}",
                    light_intensity
                ));
            }
        }

        if let Update::Set(humidity_setpoint) = self.humidity_setpoint {
            if !(0.0..0.95).contains(&humidity_setpoint) {
                return Err(anyhow!(
                    "humidity_setpoint must be between 0.0 and 0.95, got {}",
                    humidity_setpoint
                ));
            }
        }

        Ok(())
    }
}

impl ScheduledEvent {
    fn validate(&self) -> anyhow::Result<()> {
        if let Some(repeat) = &self.repeat {
            if self.start_time >= repeat.stop_time {
                return Err(anyhow!("Stop time must be after start time"));
            }
        }
        Ok(())
    }
}

fn validate_scheduled_events(events: &Vec<ScheduledEvent>) -> anyhow::Result<()> {
    for event in events {
        event.validate()?;
    }
    Ok(())
}

#[cfg(test)]
mod schedule {
    use super::*;

    #[test]
    fn evaluate() {
        let sch = Schedule {
            lights: Some(TimeRange {
                start: "08:30".parse().unwrap(),
                stop: "22:00".parse().unwrap(),
            }),
            light_intensity: Some(0.5),
            fans: vec![ScheduledEvent {
                start_time: "09:00".parse().unwrap(),
                duration_secs: 100,
                repeat: Some(RepeatInfo {
                    n_hours: 1,
                    stop_time: "22:00".parse().unwrap(),
                }),
            }],
            mist: vec![ScheduledEvent {
                start_time: "09:00".parse().unwrap(),
                duration_secs: 100,
                repeat: Some(RepeatInfo {
                    n_hours: 1,
                    stop_time: "22:00".parse().unwrap(),
                }),
            }],
            humidity_setpoint: None,
            mist_mode: HumidityMode::Manual,
        };

        assert_eq!(
            sch.evaluate("06:00".parse().unwrap()),
            ActuatorValues::default()
        );

        assert_eq!(
            sch.evaluate("09:30".parse().unwrap()),
            ActuatorValues {
                lights: 0.5,
                fans: false,
                mist: false,
            }
        );

        assert_eq!(
            sch.evaluate("22:01".parse().unwrap()),
            ActuatorValues {
                lights: 0.0,
                fans: true,
                mist: true,
            }
        );
    }

    #[test]
    fn test_evaluate_scheduled_events_nonrepeating() {
        let events = vec![
            ScheduledEvent {
                start_time: "09:00".parse().unwrap(),
                duration_secs: 30,
                repeat: None,
            },
            ScheduledEvent {
                start_time: "10:00".parse().unwrap(),
                duration_secs: 30,
                repeat: None,
            },
        ];

        struct Entry<'a> {
            time: &'a str,
            on: bool,
        }
        for test in &vec![
            Entry {
                time: "09:00",
                on: true,
            },
            Entry {
                time: "09:00:29",
                on: true,
            },
            Entry {
                time: "09:00:31",
                on: false,
            },
            Entry {
                time: "19:00",
                on: false,
            },
            Entry {
                time: "10:00:10",
                on: true,
            },
            Entry {
                time: "10:00:32",
                on: false,
            },
        ] {
            assert_eq!(
                evaluate_scheduled_events(&events, test.time.parse().unwrap()),
                test.on
            );
        }
    }

    #[test]
    fn test_evaluate_scheduled_events_repeating() {
        let events = vec![ScheduledEvent {
            start_time: "09:00".parse().unwrap(),
            duration_secs: 30,
            repeat: Some(RepeatInfo {
                n_hours: 1,
                stop_time: "22:00".parse().unwrap(),
            }),
        }];

        struct Entry<'a> {
            time: &'a str,
            on: bool,
        }
        for test in &vec![
            Entry {
                time: "09:00",
                on: true,
            },
            Entry {
                time: "09:00:29",
                on: true,
            },
            Entry {
                time: "09:00:32",
                on: false,
            },
            Entry {
                time: "11:00",
                on: true,
            },
            Entry {
                time: "11:00:29",
                on: true,
            },
            Entry {
                time: "11:00:32",
                on: false,
            },
            Entry {
                time: "23:00:15",
                on: false,
            },
        ] {
            assert_eq!(
                evaluate_scheduled_events(&events, test.time.parse().unwrap()),
                test.on
            );
        }
    }
}

#[cfg(test)]
mod config_update {
    use super::*;

    #[test]
    fn update_set_one_field() {
        let mut upd = TerrariumConfigUpdate::default();
        upd.name = Update::Set("justin".to_string());
        assert_eq!(
            serde_json::to_string(&upd).unwrap(),
            "{\"name\":\"justin\"}"
        );
    }

    #[test]
    fn update_set_one_field_clear_one_field() {
        let mut upd = TerrariumConfigUpdate::default();
        upd.name = Update::Set("justin".to_string());
        upd.wifi = Update::Clear;
        assert_eq!(
            serde_json::to_string(&upd).unwrap(),
            "{\"name\":\"justin\",\"wifi\":null}"
        );
    }

    #[test]
    fn deserialize_set_one_clear_one() {
        let json = "{\"name\":\"justin\",\"wifi\":null}";
        let upd: TerrariumConfigUpdate = serde_json::from_str(json).unwrap();
        let upd_expect = TerrariumConfigUpdate {
            name: Update::Set("justin".to_string()),
            wifi: Update::Clear,
            influxdb: Update::NoChange,
            schedule: Update::NoChange,
        };
        assert_eq!(upd, upd_expect);
    }
}
