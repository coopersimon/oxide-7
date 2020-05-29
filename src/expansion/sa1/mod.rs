// SA-1: 65C816 clocked at 10.74MHz

mod bus;

use crate::{
    cpu::CPU,
    mem::rom::ROM
};

use bus::SA1Bus;

pub struct SA1 {
    cpu: CPU<SA1Bus>
}

impl SA1 {
    pub fn new(rom: ROM, lo_rom: bool) -> Self {
        let mem = SA1Bus::new(lo_rom);
        Self {
            cpu: CPU::new(mem, 2)
        }
    }
}