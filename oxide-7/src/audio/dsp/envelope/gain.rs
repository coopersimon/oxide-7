// Gain envelope setup.
use super::EnvelopeState;

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct GainSettings: u8 {
        const DIRECT        = bit!(7);
        const DIRECT_PARAM  = bits![6, 5, 4, 3, 2, 1, 0];
        const GAIN_MODE     = bits![6, 5];
        const GAIN_PARAM    = bits![4, 3, 2, 1, 0];
    }
}

impl GainSettings {
    // Gets the state for the gain.
    pub fn state(&self) -> EnvelopeState {
        const LINEAR_INCREASE: u8       = 2 << 5;
        const BENT_LINE_INCREASE: u8    = 3 << 5;
        const LINEAR_DECREASE: u8       = 0 << 5;
        const EXP_DECREASE: u8          = 1 << 5;

        if !self.contains(GainSettings::DIRECT) {
            EnvelopeState::Direct
        } else {
            match (*self & GainSettings::GAIN_MODE).bits() {    // TODO: what should "none" do here? Off or Static(_)?
                LINEAR_INCREASE     => EnvelopeState::LinearIncrease,
                BENT_LINE_INCREASE  => EnvelopeState::BentLineIncrease,
                LINEAR_DECREASE     => EnvelopeState::LinearDecrease,
                EXP_DECREASE        => EnvelopeState::ExpDecrease,
                _ => unreachable!()
            }
        }
    }

    pub fn direct_param(&self) -> i16 {
        let param = (*self & GainSettings::DIRECT_PARAM).bits() as i16;
        param * 16
    }

    pub fn gain_param(&self) -> Option<usize> {
        let param = (*self & GainSettings::GAIN_PARAM).bits();
        step_size(param)
    }
}

// Calculations:
// Param specifies the number of outputs needed before the envelope changes.
pub fn step_size(param: u8) -> Option<usize> {
    match param {
        0x00 => None,
        0x01 => Some(2048),
        0x02 => Some(1536),
        0x03 => Some(1280),
        0x04 => Some(1024),
        0x05 => Some(768),
        0x06 => Some(640),
        0x07 => Some(512),
        0x08 => Some(384),
        0x09 => Some(320),
        0x0A => Some(256),
        0x0B => Some(192),
        0x0C => Some(160),
        0x0D => Some(128),
        0x0E => Some(96),
        0x0F => Some(80),
        0x10 => Some(64),
        0x11 => Some(48),
        0x12 => Some(40),
        0x13 => Some(32),
        0x14 => Some(24),
        0x15 => Some(20),
        0x16 => Some(16),
        0x17 => Some(12),
        0x18 => Some(10),
        0x19 => Some(8),
        0x1A => Some(6),
        0x1B => Some(5),
        0x1C => Some(4),
        0x1D => Some(3),
        0x1E => Some(2),
        0x1F => Some(1),
        _ => unreachable!()
    }
}
