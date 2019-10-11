#[macro_use]
mod common;
mod constants;

mod cpu;
mod joypad;
mod mem;
mod video;
mod audio;

mod snes;

mod debug;

use chrono::{
    Duration,
    Utc
};

// Target output frame rate.
const TARGET_FRAME_RATE: usize = 15;
const FRAME_INTERVAL: f32 = 1.0 / TARGET_FRAME_RATE as f32;

// Emulated frames per second.
const NMI_PER_SECOND: usize = 60;
const NMI_INTERVAL: f32 = 1.0 / NMI_PER_SECOND as f32;
const NMI_PER_FRAME: f32 = FRAME_INTERVAL / NMI_INTERVAL;

fn main() {
    let cart_path = std::env::args().nth(1).expect("Expected ROM file path as first argument!");

    let debug_mode = std::env::args().nth(2).is_some();

    let mut snes = snes::SNES::new(&cart_path, "");

    let mut now = Utc::now();
    let frame_duration = Duration::microseconds((FRAME_INTERVAL * 1_000_000.0) as i64);
    let mut nmi_count = NMI_PER_FRAME;

    if debug_mode {
        debug::debug_mode(&mut snes.cpu);
    } else {
        loop {
            while nmi_count > 0.0 {
                // Keep going until NMI occurs.
                while !snes.step() {}
                nmi_count -= 1.0;
            }

            // Wait for a frame to pass...
            //println!("Frame time: {}ms", Utc::now().signed_duration_since(now).num_milliseconds());
            while Utc::now().signed_duration_since(now) < frame_duration {}

            snes.enable_rendering();
            nmi_count += NMI_PER_FRAME;
            now = Utc::now();
        }
    }
}
