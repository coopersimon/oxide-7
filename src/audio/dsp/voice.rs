// A single audio channel

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct ADSRLo: u8 {
        const ENABLE = bit!(7);
        const DECAY = bits![6, 5, 4];
        const ATTACK = bits![3, 2, 1, 0];
    }
}

bitflags! {
    #[derive(Default)]
    pub struct ADSRHi: u8 {
        const SUSTAIN_LEVEL = bits![7, 6, 5];
        const RELEASE = bits![4, 3, 2, 1, 0];
    }
}

const PITCH_MASK: u16 = 0x3FFF;

#[derive(Clone, Copy)]
pub struct Voice {
    left_vol:   u8, // Signed magnitude representation
    right_vol:  u8,

    pitch:      u16,

    src_num:    u8, // Lower byte of memory addr to use for sample.

    adsr_lo:    ADSRLo,
    adsr_hi:    ADSRHi,
    gain:       u8,
    fir_coef:   u8,

    // Read
    envx:       u8,
    outx:       u8,
}

impl Voice {
    pub fn new() -> Self {
        Voice {
            left_vol:   0,
            right_vol:  0,

            pitch:      0,

            src_num:    0,

            adsr_lo:    ADSRLo::default(),
            adsr_hi:    ADSRHi::default(),
            gain:       0,
            fir_coef:   0,

            envx:       0,
            outx:       0,
        }
    }

    // Uses a 4-bit address to index registers.
    pub fn read(&self, addr: u8) -> u8 {
        match addr & 0xF {
            0x0 => self.left_vol,
            0x1 => self.right_vol,
            0x2 => lo!(self.pitch),
            0x3 => hi!(self.pitch),
            0x4 => self.src_num,
            0x5 => self.adsr_lo.bits(),
            0x6 => self.adsr_hi.bits(),
            0x7 => self.gain,
            0x8 => self.envx,
            0x9 => self.outx,
            0xF => self.fir_coef,
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr & 0xF {
            0x0 => self.left_vol = data,
            0x1 => self.right_vol = data,
            0x2 => self.pitch = set_lo!(self.pitch, data),
            0x3 => self.pitch = set_hi!(self.pitch, data),
            0x4 => self.src_num = data,
            0x5 => self.adsr_lo = ADSRLo::from_bits_truncate(data),
            0x6 => self.adsr_hi = ADSRHi::from_bits_truncate(data),
            0x7 => self.gain = data,
            0xF => self.fir_coef = data,
            _ => {},
        }
    }
}