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
    pub timezone: Option<String>,
    pub influxdb: Option<influxdb::Config>,
}

impl TerrariumConfig {
    pub fn new_with_reasonable_defaults() -> Self {
        Self {
            name: Some("oasis".into()),
            wifi: None,
            schedule: Some(Schedule::new_with_reasonable_defaults()),
            timezone: None,
            influxdb: None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct WifiDetails {
    pub ssid: String,
    pub password: String,
}

// Max lengths (in bytes) that the esp32 wifi driver can hold.
pub const WIFI_SSID_MAX_LEN: usize = 32;
pub const WIFI_PASSWORD_MAX_LEN: usize = 64;

impl WifiDetails {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.ssid.is_empty() {
            return Err(anyhow!("Wifi network name can not be empty"));
        }
        if self.ssid.len() > WIFI_SSID_MAX_LEN {
            return Err(anyhow!(
                "Wifi network name must be at most {WIFI_SSID_MAX_LEN} bytes"
            ));
        }
        if self.password.len() > WIFI_PASSWORD_MAX_LEN {
            return Err(anyhow!(
                "Wifi password must be at most {WIFI_PASSWORD_MAX_LEN} bytes"
            ));
        }
        Ok(())
    }
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
    pub mist: Vec<ScheduledEvent>,
    // If true, the mister should automatically turn on to keep the terrarium's
    // humidity above @humidity_setpoint.
    pub auto_mist_enabled: bool,
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

impl Schedule {
    // The "reasonable defaults" schedule should be set so that if the user
    // never changes it, their plants will do ok.
    pub fn new_with_reasonable_defaults() -> Self {
        Self {
            // Lights on from 10am to 10pm.
            lights: Some(TimeRange {
                start: "10:00".parse().unwrap(),
                stop: "22:00".parse().unwrap(),
            }),
            light_intensity: Some(0.7),
            // Mist on for 60 seconds at 11am and again at 4pm.
            mist: vec![
                ScheduledEvent {
                    start_time: "11:00".parse().unwrap(),
                    duration_secs: 60,
                    repeat: None,
                },
                ScheduledEvent {
                    start_time: "16:00".parse().unwrap(),
                    duration_secs: 60,
                    repeat: None,
                },
            ],
            // Fans on for 2 minutes every hour during daylight hours. The fans
            // are intentionally set to come on while mist is *not* on.
            fans: vec![ScheduledEvent {
                start_time: "10:30".parse().unwrap(),
                duration_secs: 2 * 60,
                repeat: Some(RepeatInfo {
                    n_hours: 1,
                    stop_time: "22:00".parse().unwrap(),
                }),
            }],
            auto_mist_enabled: false,
            humidity_setpoint: None,
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

        match update.auto_mist_enabled {
            Update::Set(auto_mist_enabled) => self.auto_mist_enabled = auto_mist_enabled,
            Update::Clear => self.auto_mist_enabled = false,
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

        // note: the auto mist feature is handled by the controller, not the
        // schedule.
        v.mist = evaluate_scheduled_events(&self.mist, t);
        v.fans = evaluate_scheduled_events(&self.fans, t);

        v
    }
}

const SECS_PER_DAY: i64 = 24 * 3600;

fn secs_since_midnight(t: Time) -> i64 {
    t.hour() as i64 * 3600 + t.minute() as i64 * 60 + t.second() as i64
}

fn evaluate_scheduled_events(events: &Vec<ScheduledEvent>, t: Time) -> bool {
    // Note: all math here is done in seconds since midnight rather than with
    // `Time` arithmetic, which silently wraps around at midnight.
    let t = secs_since_midnight(t);

    // An occurrence starting at @start is active at @t if @t falls within
    // its duration. The schedule repeats daily, so an occurrence that runs
    // past midnight also covers the start of the (next) day.
    let is_active = |start: i64, duration_secs: u32| {
        (t - start).rem_euclid(SECS_PER_DAY) <= duration_secs as i64
    };

    for event in events {
        let mut start = secs_since_midnight(event.start_time);
        if is_active(start, event.duration_secs) {
            return true;
        }

        if let Some(repeat) = &event.repeat {
            // A zero interval would repeat forever at the same time. Treat it
            // as non-repeating (validation rejects it for new configs).
            if repeat.n_hours == 0 {
                continue;
            }
            let stop = secs_since_midnight(repeat.stop_time);
            loop {
                start += repeat.n_hours as i64 * 3600;
                if start > stop {
                    break;
                }
                if is_active(start, event.duration_secs) {
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
    pub timezone: Update<String>,
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
    pub auto_mist_enabled: Update<bool>,
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
        if let Update::Set(wifi) = &self.wifi {
            wifi.validate()?;
        }
        if let Update::Set(timezone) = &self.timezone {
            let _tz = jiff::tz::TimeZone::get(timezone)?;
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
            if repeat.n_hours == 0 {
                return Err(anyhow!("Repeat interval must be at least 1 hour"));
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
            auto_mist_enabled: false,
            humidity_setpoint: None,
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

    fn repeating_event(
        start: &str,
        duration_secs: u32,
        n_hours: u32,
        stop: &str,
    ) -> ScheduledEvent {
        ScheduledEvent {
            start_time: start.parse().unwrap(),
            duration_secs,
            repeat: Some(RepeatInfo {
                n_hours,
                stop_time: stop.parse().unwrap(),
            }),
        }
    }

    // These used to loop forever because `Time` addition wraps at midnight,
    // so the repeat never passed the stop time.
    #[test]
    fn test_evaluate_scheduled_events_repeat_does_not_wrap() {
        for event in [
            repeating_event("10:00", 30, 2, "22:00"),
            repeating_event("10:00", 30, 1, "23:30"),
            repeating_event("10:00", 30, 3, "22:00"),
            repeating_event("10:00", 30, 24, "22:00"),
            repeating_event("10:00", 30, 0, "22:00"),
        ] {
            let events = vec![event];
            assert!(evaluate_scheduled_events(
                &events,
                "10:00:10".parse().unwrap()
            ));
            assert!(!evaluate_scheduled_events(
                &events,
                "09:00:00".parse().unwrap()
            ));
            // Must not fire after the stop time by wrapping around.
            assert!(!evaluate_scheduled_events(
                &events,
                "00:00:10".parse().unwrap()
            ));
        }

        let events = vec![repeating_event("10:00", 30, 2, "22:00")];
        assert!(evaluate_scheduled_events(
            &events,
            "22:00:10".parse().unwrap()
        ));
        assert!(!evaluate_scheduled_events(
            &events,
            "11:00:10".parse().unwrap()
        ));

        let events = vec![repeating_event("10:00", 30, 25, "22:00")];
        assert!(!evaluate_scheduled_events(
            &events,
            "11:00:10".parse().unwrap()
        ));
    }

    #[test]
    fn test_evaluate_scheduled_events_crosses_midnight() {
        let events = vec![ScheduledEvent {
            start_time: "23:59:30".parse().unwrap(),
            duration_secs: 60,
            repeat: None,
        }];
        assert!(!evaluate_scheduled_events(
            &events,
            "23:59:00".parse().unwrap()
        ));
        assert!(evaluate_scheduled_events(
            &events,
            "23:59:45".parse().unwrap()
        ));
        assert!(evaluate_scheduled_events(
            &events,
            "00:00:10".parse().unwrap()
        ));
        assert!(evaluate_scheduled_events(
            &events,
            "00:00:30".parse().unwrap()
        ));
        assert!(!evaluate_scheduled_events(
            &events,
            "00:00:31".parse().unwrap()
        ));

        // A repeat whose last occurrence runs past midnight.
        let events = vec![repeating_event("21:00", 7200, 2, "23:00")];
        assert!(evaluate_scheduled_events(&events, "00:30".parse().unwrap()));
        assert!(!evaluate_scheduled_events(
            &events,
            "01:30".parse().unwrap()
        ));
    }

    #[test]
    fn validate_rejects_zero_repeat_interval() {
        assert!(repeating_event("10:00", 30, 0, "22:00").validate().is_err());
        assert!(repeating_event("10:00", 30, 1, "22:00").validate().is_ok());
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
            schedule: Update::NoChange,
            timezone: Update::NoChange,
            influxdb: Update::NoChange,
        };
        assert_eq!(upd, upd_expect);
    }

    #[test]
    fn validate_wifi_lengths() {
        let check = |ssid: &str, password: &str| {
            let mut upd = TerrariumConfigUpdate::default();
            upd.wifi = Update::Set(WifiDetails {
                ssid: ssid.to_string(),
                password: password.to_string(),
            });
            upd.validate().is_ok()
        };
        assert!(check("mynetwork", "hunter22"));
        assert!(check(&"s".repeat(32), &"p".repeat(64)));
        assert!(!check("", "hunter22"));
        assert!(!check(&"s".repeat(33), "hunter22"));
        assert!(!check("mynetwork", &"p".repeat(65)));
    }
}
