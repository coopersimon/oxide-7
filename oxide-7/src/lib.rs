#[macro_use]
mod common;
mod constants;

mod cpu;
mod joypad;
mod mem;
mod video;
mod audio;
mod expansion;

#[cfg(feature = "debug")]
pub mod debug;

use audio::Resampler;
use cpu::CPU;
use mem::AddrBusA;
use video::RenderTarget;

use std::sync::{
    Arc, Mutex
};

/// Size of destination buffer in bytes (R8G8B8A8 format).
pub const FRAME_BUFFER_SIZE: usize = 512 * 224 * 4;

/// Joypad buttons.
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    Start,
    Select,
    L,
    R
}

/// A SNES.
pub struct SNES {
    cpu:    CPU<AddrBusA>,    // CPU, along with mem bus and devices

    frame:  RenderTarget
}

impl SNES {
    /// Construct a new SNES with a cartridge inserted.
    pub fn new(cart_path: &str, save_path: &str, dsp_rom_path: Option<&str>) -> Self {
        let bus = AddrBusA::new(cart_path, save_path, dsp_rom_path);
        let cpu = CPU::new(bus, constants::timing::INTERNAL_OP);

        SNES {
            cpu: cpu,

            frame: Arc::new(Mutex::new(Box::new([0; FRAME_BUFFER_SIZE])))
        }
    }

    /// Call at 60fps.
    pub fn frame(&mut self, frame: &mut [u8]) {
        // frame update?
        self.cpu.start_frame(self.frame.clone());

        // When NMI is triggered, disable rendering of new frames.
        while !self.cpu.step() {}
        //self.cpu.enable_rendering(false);

        let frame_in = self.frame.lock().unwrap();
        frame.copy_from_slice(&(*frame_in));
    }

    /// Call this at the start to enable audio.
    /// It creates a SNESAudioHandler that can be sent to the audio thread.
    pub fn enable_audio(&mut self, sample_rate: f64) -> SNESAudioHandler {
        let rx = self.cpu.get_audio_rx().expect("Audio already enabled!");

        SNESAudioHandler {
            resampler: Resampler::new(rx, sample_rate),
        }
    }

    /// Sets a button on the specified joypad.
    pub fn set_button(&mut self, button: Button, val: bool, joypad: usize) {
        use joypad::Button as JB;
        self.cpu.set_buttons(match button {
            Button::Up      => JB::UP,
            Button::Down    => JB::DOWN,
            Button::Left    => JB::LEFT,
            Button::Right   => JB::RIGHT,
            Button::A       => JB::A,
            Button::B       => JB::B,
            Button::X       => JB::X,
            Button::Y       => JB::Y,
            Button::Start   => JB::START,
            Button::Select  => JB::SELECT,
            Button::L       => JB::L,
            Button::R       => JB::R
        }, val, joypad);
    }

    /// Get the name of the ROM currently running.
    pub fn rom_name(&self) -> String {
        self.cpu.rom_name()
    }
}

/// Created by a SNES.
/// Call to receive the 
pub struct SNESAudioHandler {
    resampler:    Resampler,
}

impl SNESAudioHandler {
    /// Fill the provided buffer with samples.
    /// The format is PCM interleaved stereo.
    pub fn get_audio_packet(&mut self, buffer: &mut [f32]) {
        for (o_frame, i_frame) in buffer.chunks_exact_mut(2).zip(&mut self.resampler) {
            for (o, i) in o_frame.iter_mut().zip(i_frame.iter()) {
                *o = *i;
            }
        }
    }
}

// Debug
#[cfg(feature = "debug")]
impl SNES {
    // Capture the state of the internal registers.
    pub fn get_state(&self) -> crate::debug::CPUState {
        self.cpu.get_state()
    }

    // Read a memory address. Note this may affect the internal value!
    pub fn get_mem_at(&mut self, addr: u32) -> u8 {
        self.cpu.get_mem_at(addr)
    }

    // Get the instruction at the current PC, with the next 3 bytes for context.
    pub fn get_instr(&mut self) -> [u8; 4] {
        self.cpu.get_instr()
    }

    pub fn start_frame(&mut self) {
        self.cpu.start_frame(self.frame.clone());
    }

    // Step the device by one CPU cycle.
    pub fn step(&mut self) -> bool {
        self.cpu.step()
    }

    pub fn show_frame(&mut self, frame: &mut [u8]) {
        let frame_in = self.frame.lock().unwrap();
        frame.copy_from_slice(&(*frame_in));
    }
}