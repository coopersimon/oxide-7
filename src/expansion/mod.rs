// Expansion chips found in certain carts.

mod dsp;

pub use dsp::DSP;

pub trait Expansion {
    fn read(&mut self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, data: u8);

    fn clock(&mut self, cycles: usize);
}

impl Expansion for DSP {
    fn read(&mut self, addr: u32) -> u8 {
        match addr {
            0 => self.read_dr(),
            1 => self.read_sr(),
            _ => unreachable!()
        }
    }

    fn write(&mut self, addr: u32, data: u8) {
        match addr {
            0 => self.write_dr(data),
            1 => self.write_sr(data),
            _ => unreachable!()
        }
    }

    fn clock(&mut self, cycles: usize) {
        // DSP = 8_192_000
        const DSP_CLOCK_RATE: usize = 8_192_000;
        const CLOCK_RATIO: f64 = (DSP_CLOCK_RATE as f64) / (crate::constants::timing::MASTER_HZ as f64);

        self.cycle_fill += (cycles as f64) * CLOCK_RATIO;

        while self.cycle_fill > 0.0 {
            self.cycle_fill -= 1.0;
            self.step();
        }
    }
}