use crate::types::{ActuatorValues, SensorValues, TerrariumState};

// Interface for terrarium. One implementation of this is a dummy that allows
// code to be tested on your pc and one implementation runs only on the esp32
// for the real terrarium.
pub trait Terrarium {
    fn set_lights(&mut self, val: f32);
    // Like set_lights, but the LEDs are slowly faded from their current value
    // to the new one. This function returns immediately - it does not wait for
    // the fade to finish.
    //
    // TODO: implement an async version of this that continues when the fade is
    // done
    fn set_lights_with_fade(&mut self, val: f32, fade_ms: i32);
    fn get_lights(&self) -> f32;

    fn set_mist(&mut self, on: bool);
    fn get_mist(&self) -> bool;

    fn set_fans(&mut self, on: bool);
    fn get_fans(&self) -> bool;

    // TODO: make this return a Result<> since it can fail
    fn read_sensors(&mut self) -> Option<SensorValues>;

    fn read_cpu_temp(&mut self) -> Option<f32>;
}

pub fn print_terrarium_info(t: &mut dyn Terrarium) {
    let ts = get_terrarium_state(t);
    print_terrarium_state(&ts);
}

// Temperature conversion.
fn c_to_f(c: f32) -> f32 {
    c * 1.8 + 32.0
}

pub fn print_terrarium_state(ts: &TerrariumState) {
    println!("Lights: {:.1}", ts.actuators.lights);
    println!("Mist:   {}", ts.actuators.mist);
    println!("Fans:    {}", ts.actuators.fans);
    if let Some(sens) = ts.sensors {
        println!("Temp:   {:.1}C/{:.1}F", sens.temp, c_to_f(sens.temp));
        println!("Humid:  {:.1}%", sens.humid * 100.0);
    } else {
        println!("<sensor read error>");
    }
    if let Some(temp) = ts.cpu_temp {
        println!("CPU Temp: {:.1}C/{:.1}F", temp, c_to_f(temp));
    } else {
        println!("<cpu temp unknown>");
    }
}

pub fn get_terrarium_state(t: &mut dyn Terrarium) -> TerrariumState {
    TerrariumState {
        actuators: ActuatorValues {
            lights: t.get_lights(),
            mist: t.get_mist(),
            fans: t.get_fans(),
        },
        sensors: t.read_sensors(),
        cpu_temp: t.read_cpu_temp(),
    }
}

// FakeTerrarium implements the Terrarium interface and is used for testing.
pub struct FakeTerrarium {
    pub state: TerrariumState,
}

impl FakeTerrarium {
    pub fn new() -> Self {
        Self {
            state: TerrariumState {
                actuators: ActuatorValues::default(),
                sensors: Some(SensorValues {
                    temp: 22.0,
                    humid: 0.8,
                }),
                cpu_temp: None,
            },
        }
    }
}

impl Default for FakeTerrarium {
    fn default() -> Self {
        Self::new()
    }
}

impl Terrarium for FakeTerrarium {
    fn set_lights(&mut self, val: f32) {
        self.set_lights_with_fade(val, 0);
    }
    fn set_lights_with_fade(&mut self, val: f32, _fade_ms: i32) {
        self.state.actuators.lights = val;
    }
    fn get_lights(&self) -> f32 {
        self.state.actuators.lights
    }

    fn set_mist(&mut self, on: bool) {
        self.state.actuators.mist = on;
    }
    fn get_mist(&self) -> bool {
        self.state.actuators.mist
    }

    fn set_fans(&mut self, on: bool) {
        self.state.actuators.fans = on;
    }
    fn get_fans(&self) -> bool {
        self.state.actuators.fans
    }

    fn read_sensors(&mut self) -> Option<SensorValues> {
        self.state.sensors
    }

    fn read_cpu_temp(&mut self) -> Option<f32> {
        self.state.cpu_temp
    }
}

#[cfg(test)]
mod fake_terrarium {
    use super::*;

    #[test]
    fn set_lights() {
        let t: &mut dyn Terrarium = &mut FakeTerrarium::new();
        assert_eq!(0.0, t.get_lights());
        t.set_lights(0.5);
        assert_eq!(0.5, t.get_lights());
    }
}
