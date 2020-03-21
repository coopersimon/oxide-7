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
    // Step size (0 -> 1)
    pub fn attack(&self, sample_rate: f64) -> Option<f64> {
        (match (*self & ADSRSettings::ATTACK).bits() {
            0x0 => Some(0.064),
            0x1 => Some(0.040),
            0x2 => Some(0.024),
            0x3 => Some(0.016),
            0x4 => Some(0.010),
            0x5 => Some(0.006),
            0x6 => Some(0.004),
            0x7 => Some(0.0025),
            0x8 => Some(0.0015),
            0x9 => Some(0.00100),
            0xA => Some(0.000625),
            0xB => Some(0.000375),
            0xC => Some(0.0002500),
            0xD => Some(0.00015625),
            0xE => Some(0.00009375),
            0xF => None,
            _ => unreachable!()
        }).map(|factor| factor * sample_rate)
    }

    // Gives the number of samples to go from 1 to sustain level.
    pub fn decay(&self, sample_rate: f64) -> f64 {
        (match (*self & ADSRSettings::DECAY).bits() >> 4 {
            0x0 => 0.0020,
            0x1 => 0.00125,
            0x2 => 0.00075,
            0x3 => 0.000500,
            0x4 => 0.0003125,
            0x5 => 0.0001875,
            0x6 => 0.00012500,
            0x7 => 0.00006250,
            _ => unreachable!()
        }) * sample_rate
    }

    // Gives the fraction to sustain at.
    pub fn sustain_level(&self) -> f32 {
        match (*self & ADSRSettings::SUSTAIN_LEVEL).bits() >> 13 {
            0x0 => 1.0 / 8.0,
            0x1 => 2.0 / 8.0,
            0x2 => 3.0 / 8.0,
            0x3 => 4.0 / 8.0,
            0x4 => 5.0 / 8.0,
            0x5 => 6.0 / 8.0,
            0x6 => 7.0 / 8.0,
            0x7 => 8.0 / 8.0,
            _ => unreachable!()
        }
    }

    pub fn sustain_release(&self, sample_rate: f64) -> Option<f64> {
        let sr = (*self & ADSRSettings::RELEASE).bits() >> 8;
        super::gain::stepFactor(sr as u8, sample_rate)
    }
}
