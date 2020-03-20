// ADSR constants and related objects
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct ADSRSettings: u16 {
        const SUSTAIN_LEVEL = bits![15, 14, 13];
        const RELEASE = bits![12, 11, 10, 9, 8];
        const ENABLE = bit!(7);
        const DECAY = bits![6, 5, 4];
        const ATTACK = bits![3, 2, 1, 0];
    }
}

impl ADSRSettings {
    // Gives the number of samples to go from 0 to 1.
    pub fn attack(&self, sample_rate: f32) -> usize {
        (sample_rate * match (*self & ADSRSettings::ATTACK).bits() {
            0x0 => 4.1,
            0x1 => 2.5,
            0x2 => 1.5,
            0x3 => 1.0,
            0x4 => 0.64,
            0x5 => 0.38,
            0x6 => 0.26,
            0x7 => 0.16,
            0x8 => 0.096,
            0x9 => 0.064,
            0xA => 0.040,
            0xB => 0.024,
            0xC => 0.016,
            0xD => 0.010,
            0xE => 0.006,
            0xF => 0.0,
        }) as usize
    }

    // Gives the number of samples to go from 1 to sustain level.
    pub fn decay(&self, sample_rate: f32) -> usize {
        (sample_rate * match (*self & ADSRSettings::DECAY).bits() >> 4 {
            0x0 => 1.2,
            0x1 => 0.74,
            0x2 => 0.44,
            0x3 => 0.29,
            0x4 => 0.18,
            0x5 => 0.11,
            0x6 => 0.074,
            0x7 => 0.037,
        }) as usize
    }

    // Gives the fraction to sustain at.
    pub fn sustain(&self) -> f32 {
        match (*self & ADSRSettings::SUSTAIN_LEVEL).bits() >> 13 {
            0x0 => 1.0 / 8.0,
            0x1 => 2.0 / 8.0,
            0x2 => 3.0 / 8.0,
            0x3 => 4.0 / 8.0,
            0x4 => 5.0 / 8.0,
            0x5 => 6.0 / 8.0,
            0x6 => 7.0 / 8.0,
            0x7 => 8.0 / 8.0,
        }
    }

    // Gives the number of samples to go from sustain level to 0.
    pub fn release(&self, sample_rate: f32) -> Option<usize> {
        match (*self & ADSRSettings::RELEASE).bits() >> 13 {
            0x00 => None,
            0x01 => Some(38.0),
            0x02 => Some(28.0),
            0x03 => Some(24.0),
            0x04 => Some(19.0),
            0x05 => Some(14.0),
            0x06 => Some(12.0),
            0x07 => Some(9.4),
            0x08 => Some(7.1),
            0x09 => Some(5.9),
            0x0A => Some(4.7),
            0x0B => Some(3.5),
            0x0C => Some(2.9),
            0x0D => Some(2.4),
            0x0E => Some(1.8),
            0x0F => Some(1.5),
            0x10 => Some(1.2),
            0x11 => Some(0.88),
            0x12 => Some(0.74),
            0x13 => Some(0.59),
            0x14 => Some(0.44),
            0x15 => Some(0.37),
            0x16 => Some(0.29),
            0x17 => Some(0.22),
            0x18 => Some(0.18),
            0x19 => Some(0.15),
            0x1A => Some(0.11),
            0x1B => Some(0.092),
            0x1C => Some(0.074),
            0x1D => Some(0.055),
            0x1E => Some(0.037),
            0x1F => Some(0.018),
        }.and_then(|val| Some((val * sample_rate) as usize))
    }
}
