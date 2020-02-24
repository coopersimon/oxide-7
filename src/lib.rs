#[macro_use]
mod common;
mod constants;

mod cpu;
mod joypad;
mod mem;
mod video;
mod audio;

#[cfg(feature = "debug")]
pub mod debug;

use cpu::CPU;
use mem::MemBus;

// Joypad buttons.
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

pub struct SNES {
    cpu:    CPU,    // CPU, along with mem bus and devices
}

impl SNES {
    // Construct a new SNES with a cartridge inserted.
    pub fn new(cart_path: &str, _save_path: &str, events_loop: &winit::EventsLoop) -> Self {
        let bus = MemBus::new(cart_path, events_loop);
        let cpu = CPU::new(bus);

        SNES {
            cpu: cpu,
        }
    }

    // Step the device by one CPU cycle.
    pub fn step(&mut self) -> bool {
        // When NMI is triggered, disable rendering of new frames.
        if self.cpu.step() {
            self.cpu.enable_rendering(false);
            true
        } else {
            false
        }
    }

    // Re-enable rendering of frames.
    pub fn enable_rendering(&mut self) {
        self.cpu.enable_rendering(true);
    }

    // Sets a button on the specified joypad.
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
}