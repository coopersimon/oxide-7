// SA-1: 65C816 clocked at 10.74MHz

mod mem;

use super::Expansion;
use crate::{
    common::Interrupt,
    cpu::CPU,
    mem::rom::ROM
};

use mem::SA1Bus;

pub struct SA1 {
    cpu: CPU<SA1Bus>,
    cycle_count: isize,
}

impl SA1 {
    pub fn new(rom: ROM, lo_rom: bool) -> Self {
        let mem = SA1Bus::new(rom, lo_rom);
        Self {
            cpu: CPU::new(mem, 2),
            cycle_count: 0,
        }
    }
}

impl Expansion for SA1 {
    fn read(&mut self, bank: u8, addr: u16) -> u8 {
        self.cpu.get_bus().snes_read(bank, addr)
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) {
        self.cpu.get_bus().snes_write(bank, addr, data);
    }

    fn clock(&mut self, cycles: usize) -> Interrupt {
        // 2 Master cycles = 1 SA-1 cycle.
        self.cycle_count += cycles as isize;
        while self.cycle_count > 0 {
            self.cpu.step();
            self.cycle_count -= self.cpu.get_bus().get_cycle_count() as isize;
        }

        self.cpu.get_bus().check_snes_interrupts()
    }
}