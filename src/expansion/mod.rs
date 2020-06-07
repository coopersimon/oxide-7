// Expansion chips found in certain carts.

mod dsp;
mod sa1;
mod superfx;

use crate::common::Interrupt;

pub use dsp::DSP;
pub use sa1::SA1;
pub use superfx::SuperFX;

pub trait Expansion {
    fn read(&mut self, bank: u8, addr: u16) -> u8;
    fn write(&mut self, bank: u8, addr: u16, data: u8);

    fn clock(&mut self, cycles: usize) -> Interrupt;
    fn flush(&mut self) {}
}

impl Expansion for DSP {
    fn read(&mut self, bank: u8, _addr: u16) -> u8 {
        match bank {
            0 => self.read_dr(),
            1 => self.read_sr(),
            _ => unreachable!()
        }
    }

    fn write(&mut self, bank: u8, _addr: u16, data: u8) {
        match bank {
            0 => self.write_dr(data),
            1 => self.write_sr(data),
            _ => unreachable!()
        }
    }

    fn clock(&mut self, cycles: usize) -> Interrupt {
        const DSP_CLOCK_RATE: usize = 8_192_000;
        const CLOCK_RATIO: f64 = (DSP_CLOCK_RATE as f64) / (crate::constants::timing::MASTER_HZ as f64);

        self.cycle_fill += (cycles as f64) * CLOCK_RATIO;

        while self.cycle_fill > 1.0 {
            self.cycle_fill -= 1.0;
            self.step();
        }

        Interrupt::default()
    }
}
