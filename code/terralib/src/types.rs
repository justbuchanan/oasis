use serde;
use serde::{Deserialize, Serialize};

pub const FANS: &str = "fans";
pub const MIST: &str = "mist";
pub const LIGHTS: &str = "lights";

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
pub enum UpdateValue {
    Bool(bool),
    Float(f32),
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct UpdateItem {
    pub name: String,
    pub value: UpdateValue,
    pub duration_secs: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct UpdateData {
    pub updates: Vec<UpdateItem>,
}
