// Collection of components

use winit::EventsLoop;

use crate::{
    cpu::CPU,
    mem::MemBus
    
};

pub struct SNES {
    pub cpu: CPU,    // CPU, along with mem bus and devices
}

impl SNES {
    // Construct a new SNES with a cartridge inserted.
    pub fn new(cart_path: &str, save_path: &str) -> Self {
        let bus = MemBus::new(cart_path);
        let cpu = CPU::new(bus);

        SNES {
            cpu: cpu,
        }
    }

    // Step the device by one CPU cycle.
    pub fn step(&mut self) {
        self.cpu.step();
    }
}