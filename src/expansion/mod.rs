// Expansion chips found in certain carts.

mod dsp;
mod sa1;

use crate::common::Interrupt;

pub use dsp::DSP;
pub use sa1::SA1;

pub trait Expansion {
    fn read(&mut self, bank: u8, addr: u16) -> u8;
    fn write(&mut self, bank: u8, addr: u16, data: u8);

    fn clock(&mut self, cycles: usize) -> Interrupt;
}

impl Expansion for DSP {
    fn read(&mut self, bank: u8, addr: u16) -> u8 {
        match bank {
            0 => self.read_dr(),
            1 => self.read_sr(),
            _ => unreachable!()
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) {
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

impl Expansion for SA1 {
    fn read(&mut self, bank: u8, addr: u16) -> u8 {
        match addr {
            0x2300 => 0,    // SNES CPU flag read
            0x230E => 0,    // SNES VC
            _ => 0,
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) {
        match addr {
            0x2200 => {},   // SA-1 CPU control
            0x2201 => {},   // Int enable
            0x2202 => {},   // Int clear
            0x2203 => {},   // SA-1 RST vector LSB
            0x2204 => {},   // SA-1 RST vector MSB
            0x2205 => {},   // SA-1 NMI vector LSB
            0x2206 => {},   // SA-1 NMI vector MSB
            0x2207 => {},   // SA-1 IRQ vector LSB
            0x2208 => {},   // SA-1 IRQ vector MSB

            0x2220 => {},   // MMC Bank C
            0x2221 => {},   // MMC Bank D
            0x2222 => {},   // MMC Bank E
            0x2223 => {},   // MMC Bank F
            0x2224 => {},   // BMAPS
            0x2226 => {},   // BW-RAM Write enable
            0x2228 => {},   // BW-RAM write-protected area
            0x2229 => {},   // I-RAM Write-protection

            _ => {}
        }
    }

    fn clock(&mut self, cycles: usize) -> Interrupt {
        // 2 Master cycles = 1 SA-1 cycle.
        Interrupt::default()
    }
}