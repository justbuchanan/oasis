use core::ffi::c_void;
use embassy_futures::select;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_idf_svc::sys;
use std::sync::{Arc, Mutex};
use terralib::terrarium::Terrarium;

const BASELINE_BRIGHTNESS: f32 = 0.15; // Default dim level when not flashing

// Show a lightning effect by flashing the lights at random intervals and random
// brightness levels. This function was originally written by claude, but
// heavily adapted:
// https://claude.ai/chat/a1b308b3-21f7-4812-adda-2a49af21548a.
//
// TODO: add a duration - as-is, this runs forever
#[embassy_executor::task]
pub async fn lightning(terrarium_ptr: Arc<Mutex<dyn Terrarium>>) {
    // Slowly fade down to dim lighting
    terrarium_ptr
        .lock()
        .unwrap()
        .set_lights_with_fade(BASELINE_BRIGHTNESS, 3000);
    Timer::after(Duration::from_millis(3000)).await;

    // Start water
    terrarium_ptr.lock().unwrap().set_mist(true);

    loop {
        // Choose a lightning type based on random value
        let random_val = random() % 100;
        if random_val < 40 {
            // 40% chance for single flash
            // Quick ramp up to high brightness
            let flash_brightness = 0.8 + (random() % 20) as f32 / 100.0; // 0.8-1.0
            terrarium_ptr.lock().unwrap().set_lights(flash_brightness);

            // Short hold at peak
            let hold_ms = 50 + (random() % 100);
            Timer::after(Duration::from_millis(hold_ms)).await;
        } else if random_val < 70 {
            // 30% chance for double flash
            // First flash
            let first_brightness = 0.7 + (random() % 20) as f32 / 100.0; // 0.7-0.9
            terrarium_ptr.lock().unwrap().set_lights(first_brightness);

            // Short hold at first peak
            let hold1_ms = 50 + (random() % 50);
            Timer::after(Duration::from_millis(hold1_ms)).await;

            // Brief dim between flashes
            let dim_brightness = 0.2 + (random() % 10) as f32 / 100.0; // 0.2-0.3
            terrarium_ptr.lock().unwrap().set_lights(dim_brightness);

            // Very brief pause
            let pause_ms = 30 + (random() % 50);
            Timer::after(Duration::from_millis(pause_ms)).await;

            // Second flash (brighter)
            let second_brightness = 0.85 + (random() % 15) as f32 / 100.0; // 0.85-1.0
            terrarium_ptr.lock().unwrap().set_lights(second_brightness);

            // Hold at second peak
            let hold2_ms = 100 + (random() % 100);
            Timer::after(Duration::from_millis(hold2_ms)).await;
        } else if random_val < 90 {
            // 20% chance for complex flash
            // Multiple flashes of varying intensity
            let num_flashes = 3 + (random() % 4); // 3-6 flashes

            for _ in 0..num_flashes {
                // Random brightness for this flash
                let flash_brightness = 0.5 + (random() % 50) as f32 / 100.0; // 0.5-1.0
                terrarium_ptr.lock().unwrap().set_lights(flash_brightness);

                // Short hold at peak
                let hold_ms = 30 + (random() % 90);
                Timer::after(Duration::from_millis(hold_ms)).await;

                // Dim between flashes but not all the way to baseline
                let dim_level = 0.15 + (random() % 15) as f32 / 100.0; // 0.15-0.3
                terrarium_ptr.lock().unwrap().set_lights(dim_level);

                // Brief pause between flashes
                let pause_ms = 20 + (random() % 80);
                Timer::after(Duration::from_millis(pause_ms)).await;
            }
        } else {
            // 10% chance for distant flash
            // Dimmer flash for distant lightning
            let flash_brightness = 0.3 + (random() % 20) as f32 / 100.0; // 0.3-0.5
            terrarium_ptr.lock().unwrap().set_lights(flash_brightness);

            // Longer hold for distant lightning
            let hold_ms = 200 + (random() % 200);
            Timer::after(Duration::from_millis(hold_ms)).await;

            // Gradual fade
            let mid_brightness = 0.15 + (random() % 10) as f32 / 100.0; // 0.15-0.25
            terrarium_ptr.lock().unwrap().set_lights(mid_brightness);

            // Extra fade step
            let fade_ms = 100 + (random() % 100);
            Timer::after(Duration::from_millis(fade_ms)).await;
        }

        // Return to baseline
        terrarium_ptr
            .lock()
            .unwrap()
            .set_lights(BASELINE_BRIGHTNESS);

        // Random pause between 1.5 and 8 seconds
        let pause_ms = 1000 + (random() % 3500);
        Timer::after(Duration::from_millis(pause_ms)).await;
    }

    // TODO: re-enable when above loop terminates
    // terrarium.set_mist(false);

    // TODO: fade lights back to where they were
}

// Simple random number generator since we can't use the stdlib rand
fn random() -> u64 {
    unsafe {
        let mut out: u32 = 0;
        sys::esp_fill_random(
            &mut out as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>(),
        );
        out as u64
    }
}

// Context object used to cancel an async operation and optionally wait for it to complete.
pub struct CancelContext {
    cancel_signal: Signal<NoopRawMutex, ()>,
    done_signal: Signal<NoopRawMutex, ()>,
}

impl Default for CancelContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CancelContext {
    pub fn new() -> Self {
        Self {
            cancel_signal: Signal::new(),
            done_signal: Signal::new(),
        }
    }

    pub fn cancel(&self) {
        self.cancel_signal.signal(());
    }

    pub async fn cancel_and_wait(&self) {
        self.cancel();
        self.done_signal.wait().await
    }

    pub async fn wait_for_cancel(&self) {
        self.cancel_signal.wait().await
    }

    pub fn done(&self) {
        self.done_signal.signal(());
    }
}

// The "breathe" effect fades the lights up and down repeatedly. The parameters
// determine the brightness range, speed of change, and total duration. The
// CancelContext allows it to be cancelled early.
#[embassy_executor::task]
pub async fn breathe(
    terrarium: Arc<Mutex<dyn Terrarium>>,
    min: f32,
    max: f32,
    period_ms: u32,
    ctx: Arc<CancelContext>,
) {
    let lights_before = terrarium.lock().unwrap().get_lights();
    let mut up = true;
    loop {
        let target = if up { max } else { min };
        let t: u64 = (period_ms / 2).into();
        terrarium
            .lock()
            .unwrap()
            .set_lights_with_fade(target, t as i32);
        match select::select(
            Timer::after(Duration::from_millis(t)),
            ctx.wait_for_cancel(),
        )
        .await
        {
            select::Either::First(_) => {
                // timer finished, continue looping
            }
            select::Either::Second(_) => {
                // received cancel signal
                terrarium.lock().unwrap().set_lights(lights_before);
                ctx.done();
                return;
            }
        }
        up = !up;
    }
}
