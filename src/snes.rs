// Collection of components

use winit::EventsLoop;

use crate::{
    cpu::CPU,
    mem::MemBus
    
};

pub struct SNES {
    pub cpu:        CPU,        // CPU, along with mem bus and devices
    events_loop:    EventsLoop, // EventsLoop
    // Joypads
    // Audio thread
    // Video thread (?)
}

impl SNES {
    // Construct a new SNES with a cartridge inserted.
    pub fn new(cart_path: &str, save_path: &str) -> Self {
        let bus = MemBus::new(cart_path);
        let cpu = CPU::new(bus);

        let events_loop = EventsLoop::new();

        SNES {
            cpu:            cpu,
            events_loop:    events_loop
        }
    }

    // Step the device by one CPU cycle.
    pub fn step(&mut self) {
        self.cpu.step();
    }
}