// Memory bus attached to SA-1.

use crate::{
    common::Interrupt,
    mem::MemBus
};

pub struct SA1Bus {

    // Misc
    lo_rom: bool    // ROM uses a lo or hi mapping.
}

impl SA1Bus {
    pub fn new(lo_rom: bool) -> Self {
        Self {
            lo_rom: lo_rom
        }
    }
}

impl MemBus for SA1Bus {
    fn read(&mut self, addr: u32) -> (u8, usize) {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        (0,0)
    }

    fn write(&mut self, addr: u32, data: u8) -> usize {
        0
    }

    fn clock(&mut self, cycles: usize) -> Interrupt {
        Interrupt::default()
    }
}