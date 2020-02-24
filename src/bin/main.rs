mod avg;
mod debug;

use chrono::{
    Duration,
    Utc
};

use oxide7::*;

use winit::{
    EventsLoop,
    Event,
    WindowEvent,
    VirtualKeyCode,
    ElementState
};

// Target output frame rate.
const TARGET_FRAME_RATE: usize = 60;
const FRAME_INTERVAL: f32 = 1.0 / TARGET_FRAME_RATE as f32;

// Emulated frames per second.
const NMI_PER_SECOND: usize = 60;
const NMI_INTERVAL: f32 = 1.0 / NMI_PER_SECOND as f32;
const NMI_PER_FRAME: f32 = FRAME_INTERVAL / NMI_INTERVAL;

fn main() {
    let cart_path = std::env::args().nth(1).expect("Expected ROM file path as first argument!");

    let debug_mode = std::env::args().nth(2).is_some();

    let mut events_loop = EventsLoop::new();
    let mut snes = SNES::new(&cart_path, "", &events_loop);

    let mut now = Utc::now();
    let frame_duration = Duration::microseconds((FRAME_INTERVAL * 1_000_000.0) as i64);
    let mut nmi_count = NMI_PER_FRAME;

    let mut averager = avg::Averager::new(100);

    if debug_mode {
        debug::debug_mode(&mut snes);
    } else {
        loop {
            while nmi_count > 0.0 {
                // Keep going until NMI occurs.
                while !snes.step() {}
                nmi_count -= 1.0;
            }

            // Wait for a frame to pass...
            let frame_time = Utc::now().signed_duration_since(now).num_milliseconds();
            //println!("Frame time: {}ms", frame_time);
            averager.add(frame_time as usize);
            println!("Frame average: {}ms", averager.get_avg());
            while Utc::now().signed_duration_since(now) < frame_duration {}

            snes.enable_rendering();
            nmi_count += NMI_PER_FRAME;
            now = Utc::now();
        }
    }
}

// Internal on video thread.
fn read_events(events_loop: &mut EventsLoop, snes: &mut SNES) {
    events_loop.poll_events(|e| {
        match e {
            Event::WindowEvent {
                window_id: _,
                event: w,
            } => match w {
                WindowEvent::CloseRequested => {
                    ::std::process::exit(0);
                },
                WindowEvent::KeyboardInput {
                    device_id: _,
                    input: k,
                } => {
                    let pressed = match k.state {
                        ElementState::Pressed => true,
                        ElementState::Released => false,
                    };
                    match k.virtual_keycode {
                        Some(VirtualKeyCode::X)         => snes.set_button(Button::A, pressed, 0),
                        Some(VirtualKeyCode::Z)         => snes.set_button(Button::B, pressed, 0),
                        Some(VirtualKeyCode::Space)     => snes.set_button(Button::Select, pressed, 0),
                        Some(VirtualKeyCode::Return)    => snes.set_button(Button::Start, pressed, 0),
                        Some(VirtualKeyCode::Up)        => snes.set_button(Button::Up, pressed, 0),
                        Some(VirtualKeyCode::Down)      => snes.set_button(Button::Down, pressed, 0),
                        Some(VirtualKeyCode::Left)      => snes.set_button(Button::Left, pressed, 0),
                        Some(VirtualKeyCode::Right)     => snes.set_button(Button::Right, pressed, 0),
                        _ => {},
                    }
                },
                _ => {}
            },
            _ => {},
        }
    });
}