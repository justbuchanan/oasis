use serde;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Actuator {
    Lights,
    Fans,
    Mist,
}

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct SensorValues {
    // Temperature in degrees Celsius
    pub temp: f32,
    // Relative humidity - a value between 0.0 and 1.0
    pub humid: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct ActuatorValues {
    pub lights: f32,
    pub mist: bool,
    pub fans: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct TerrariumState {
    pub actuators: ActuatorValues,
    // TODO: use Result for the below two?
    pub sensors: Option<SensorValues>,
    pub cpu_temp: Option<f32>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
#[serde(untagged)]
pub enum ActuatorValue {
    Bool(bool),
    Float(f32),
}

// Represents a temporary override of a single actuator. For example "set the
// lights to 0.75 for 60 seconds".
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ActuatorOverride {
    pub actuator: Actuator,
    pub value: ActuatorValue,
    pub duration_secs: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ActuatorOverrideSet {
    pub updates: Vec<ActuatorOverride>,
}
