// ADSR envelope setup.
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct ADSRSettings: u16 {
        const SUSTAIN_LEVEL = bits16![15, 14, 13];
        const RELEASE = bits16![12, 11, 10, 9, 8];
        const ENABLE = bit!(7, u16);
        const DECAY = bits16![6, 5, 4];
        const ATTACK = bits16![3, 2, 1, 0];
    }
}

impl ADSRSettings {
    // Step size (0 -> MAX)
    pub fn attack(&self) -> Option<usize> {
        match (*self & ADSRSettings::ATTACK).bits() {
            0x0 => Some(2048),
            0x1 => Some(1280),
            0x2 => Some(768),
            0x3 => Some(512),
            0x4 => Some(320),
            0x5 => Some(192),
            0x6 => Some(128),
            0x7 => Some(80),
            0x8 => Some(48),
            0x9 => Some(32),
            0xA => Some(20),
            0xB => Some(12),
            0xC => Some(8),
            0xD => Some(5),
            0xE => Some(3),
            0xF => None,
            _ => unreachable!()
        }
    }

    // Gives the number of samples to go from 1 to sustain level.
    pub fn decay(&self) -> usize {
        match (*self & ADSRSettings::DECAY).bits() >> 4 {
            0x0 => 64,
            0x1 => 40,
            0x2 => 24,
            0x3 => 16,
            0x4 => 10,
            0x5 => 6,
            0x6 => 4,
            0x7 => 2,
            _ => unreachable!()
        }
    }

    // Gives the fraction to sustain at.
    pub fn sustain_level(&self) -> i16 {
        match (*self & ADSRSettings::SUSTAIN_LEVEL).bits() >> 13 {
            0x0 => 255,
            0x1 => 511,
            0x2 => 767,
            0x3 => 1023,
            0x4 => 1279,
            0x5 => 1535,
            0x6 => 1791,
            0x7 => 2047,
            _ => unreachable!()
        }
    }

    pub fn sustain_release(&self) -> Option<usize> {
        let sr = (*self & ADSRSettings::RELEASE).bits() >> 8;
        super::gain::step_size(sr as u8)
    }
}
