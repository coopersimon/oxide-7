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
    pub fn state(&self, sample_rate: f64) -> EnvelopeState {
        const LINEAR_INCREASE: u8       = 2 << 6;
        const BENT_LINE_INCREASE: u8    = 3 << 6;
        const LINEAR_DECREASE: u8       = 0 << 6;
        const EXP_DECREASE: u8          = 1 << 6;

        if !self.contains(GainSettings::DIRECT) {
            let param = (*self & GainSettings::DIRECT_PARAM).bits() as f32;
            EnvelopeState::Static(param / 127.0)
        } else {
            let param = (*self & GainSettings::GAIN_PARAM).bits();
            match (*self & GainSettings::GAIN_MODE).bits() {    // TODO: what should "none" do here? Off or Static(1.0)?
                LINEAR_INCREASE     => step_factor(param, sample_rate).map_or(EnvelopeState::Static(0.0), |v| EnvelopeState::LinearIncrease(v)),
                BENT_LINE_INCREASE  => step_factor(param, sample_rate).map_or(EnvelopeState::Static(0.0), |v| EnvelopeState::BentLineIncrease(v)),
                LINEAR_DECREASE     => step_factor(param, sample_rate).map_or(EnvelopeState::Static(1.0), |v| EnvelopeState::LinearDecrease(v)),
                EXP_DECREASE        => step_factor(param, sample_rate).map_or(EnvelopeState::Static(1.0), |v| EnvelopeState::ExpDecrease(v)),
                _ => unreachable!()
            }
        }
    }
}

// Calculations:
// Param specifies a step factor that is multiplied by the total number of steps
// (factor * nSteps) = totalTime
// totalTime * sampleRate = nSamples
// nSamples / nSteps = stepDuration
// so: factor * sampleRate = stepDuration

pub fn step_factor(param: u8, sample_rate: f64) -> Option<f64> {
    match param {
        0x00 => None,
        0x01 => Some(0.064),
        0x02 => Some(0.048),
        0x03 => Some(0.040),
        0x04 => Some(0.032),
        0x05 => Some(0.024),
        0x06 => Some(0.020),
        0x07 => Some(0.016),
        0x08 => Some(0.012),
        0x09 => Some(0.010),
        0x0A => Some(0.008),
        0x0B => Some(0.006),
        0x0C => Some(0.005),
        0x0D => Some(0.004),
        0x0E => Some(0.003),
        0x0F => Some(0.0025),
        0x10 => Some(0.0020),
        0x11 => Some(0.0015),
        0x12 => Some(0.00125),
        0x13 => Some(0.00100),
        0x14 => Some(0.00075),
        0x15 => Some(0.000625),
        0x16 => Some(0.000500),
        0x17 => Some(0.000375),
        0x18 => Some(0.0003125),
        0x19 => Some(0.0002500),
        0x1A => Some(0.0001875),
        0x1B => Some(0.00015625),
        0x1C => Some(0.00012500),
        0x1D => Some(0.00009375),
        0x1E => Some(0.00006250),
        0x1F => Some(0.00003125),
        _ => unreachable!()
    }.map(|factor| factor * sample_rate)
}
