// Collection of components

use crate::{
    cpu::CPU,
    mem::MemBus
};

pub struct SNES {
    pub cpu:    CPU,    // CPU, along with mem bus and devices
}

impl SNES {
    // Construct a new SNES with a cartridge inserted.
    pub fn new(cart_path: &str, _save_path: &str) -> Self {
        let bus = MemBus::new(cart_path);
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
}