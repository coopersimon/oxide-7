// Envelope which alters the gain of the samples.
mod adsr;
mod gain;

pub use gain::step_size;

use adsr::*;
use gain::*;

const MAX_GAIN: i16 = 0x7E0;
const MAX_ATTACK: i16 = 0x7E0;
const GAIN_STEP: i16 = 32;      // Linear increase/decrease gain adjustment per step.
const BENT_STEP: i16 = 8;     // Bent line slow increase gain adjustment per step.

const BENT_MAX: i16 = 1536;        // Point at which bent line switches from fast to slow increase.

pub struct Envelope {
    adsr:           ADSRSettings,
    gain:           GainSettings,

    state:          EnvelopeState,
    current_gain:   i16,
    count:          usize,
}

impl Envelope {
    pub fn new(adsr: u16, gain: u8) -> Self {
        let adsr_settings = ADSRSettings::from_bits_truncate(adsr);
        let gain_settings = GainSettings::from_bits_truncate(gain);

        let state = EnvelopeState::new(adsr_settings, gain_settings);
        let initial_gain = state.initial_gain();

        Envelope {
            adsr:           adsr_settings,
            gain:           gain_settings,

            state:          state,
            current_gain:   initial_gain,
            count:          0,
        }
    }

    pub fn set_adsr(&mut self, adsr: u16) {
        self.adsr = ADSRSettings::from_bits_truncate(adsr);
    }

    pub fn set_gain(&mut self, gain: u8) {
        self.gain = GainSettings::from_bits_truncate(gain);
    }

    pub fn off(&mut self) {
        self.count = 0;
        self.state = EnvelopeState::Release;
    }

    pub fn muted(&self) -> bool {
        self.state == EnvelopeState::Release && self.current_gain <= 0
    }
}

impl Iterator for Envelope {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            EnvelopeState::Attack => {
                let out = self.current_gain;
                if let Some(step_len) = self.adsr.attack() {
                    self.count += 1;
                    if self.count >= step_len {
                        self.current_gain += GAIN_STEP;
                        self.count = 0;
                        if self.current_gain >= MAX_ATTACK {
                            self.current_gain = MAX_ATTACK;
                            self.state = EnvelopeState::Decay;
                        }
                    }
                } else {
                    self.current_gain = MAX_ATTACK;
                    self.state = EnvelopeState::Decay;
                }
                Some(out)
            },
            EnvelopeState::Decay => {
                let step_len = self.adsr.decay();
                let sustain_level = self.adsr.sustain_level();
                let out = self.current_gain;
                self.count += 1;
                if self.count >= step_len {
                    let decay_factor = ((self.current_gain - 1) >> 8) + 1;
                    self.current_gain = self.current_gain - decay_factor;
                    self.count = 0;
                }
                if self.current_gain <= sustain_level {
                    self.state = EnvelopeState::Sustain;
                    self.current_gain = self.adsr.sustain_level();
                    self.count = 0;
                }
                Some(out)
            },
            EnvelopeState::Sustain => {
                let out = self.current_gain;
                if let Some(step_len) = self.adsr.sustain_release() {
                    self.count += 1;
                    if self.count >= step_len {
                        let decay_factor = ((self.current_gain - 1) >> 8) + 1;
                        self.current_gain = self.current_gain - decay_factor;
                        self.count = 0;
                    }
                    if self.current_gain <= 0 {
                        self.current_gain = 0;
                        self.count = 0;
                        self.state = EnvelopeState::Release;
                    }
                }
                Some(out)
            },
            EnvelopeState::Direct => {
                Some(self.gain.direct_param())
            },
            EnvelopeState::LinearIncrease => {
                let out = self.current_gain;
                if let Some(step_len) = self.gain.gain_param() {
                    self.count += 1;
                    if self.count >= step_len {
                        self.current_gain += GAIN_STEP;
                        self.count = 0;
                        if self.current_gain >= MAX_GAIN {
                            self.current_gain = MAX_GAIN;
                        }
                    }
                }
                Some(out)
            },
            EnvelopeState::BentLineIncrease => {
                let out = self.current_gain;
                if let Some(step_len) = self.gain.gain_param() {
                    self.count += 1;
                    if self.count >= step_len {
                        self.current_gain += if self.current_gain >= BENT_MAX {
                            BENT_STEP
                        } else {
                            GAIN_STEP
                        };
                        self.count = 0;
                        if self.current_gain >= MAX_GAIN {
                            self.current_gain = MAX_GAIN;
                        }
                    }
                }
                Some(out)
            },
            EnvelopeState::LinearDecrease => {
                let out = self.current_gain;
                if let Some(step_len) = self.gain.gain_param() {
                    self.count += 1;
                    if self.count >= step_len {
                        self.current_gain -= GAIN_STEP;
                        self.count = 0;
                        if self.current_gain <= 0 {
                            self.current_gain = 0;
                        }
                    }
                }
                Some(out)
            },
            EnvelopeState::ExpDecrease => {
                let out = self.current_gain;
                if let Some(step_len) = self.gain.gain_param() {
                    self.count += 1;
                    if self.count >= step_len {
                        let decay_factor = ((self.current_gain - 1) >> 8) + 1;
                        self.current_gain = self.current_gain - decay_factor;
                        self.count = 0;
                        if self.current_gain <= 0 {
                            self.current_gain = 0;
                        }
                    }
                }
                Some(out)
            },
            EnvelopeState::Release if self.current_gain > 0 => {
                let out = self.current_gain;
                self.current_gain -= BENT_STEP;
                Some(out)
            },
            EnvelopeState::Release => None,    // if gain <= 0
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EnvelopeState {
    // ADSR
    Attack,     // Increase of 1/64 per step.
    Decay,      // Dn = D(n-1) * (255/256) per step; D0 = 1.0 - sustainLevel; Gn = sustainLevel + Dn
    Sustain,    // Gn = G(n-1) * (255/256) per step, G0 = sustainLevel
    // GAIN
    Direct,
    LinearIncrease,     // Increase of 1/64 per step.
    BentLineIncrease,   // Increase of 1/64 until 3/4, then 1/256.
    LinearDecrease,     // Decrease of 1/64 per step.
    ExpDecrease,        // Gn = G(n-1) * (255/256) per step, G0 = 1.0.
    // GENERAL
    Release             // On key off. Decrease of 1/256 per step.
}

impl EnvelopeState {
    #[inline]
    fn new(adsr: ADSRSettings, gain: GainSettings) -> Self {
        if adsr.contains(ADSRSettings::ENABLE) {
            EnvelopeState::Attack
        } else {
            gain.state()
        }
    }

    #[inline]
    fn initial_gain(&self) -> i16 {
        use EnvelopeState::*;
        match self {
            Direct |
            Attack |
            LinearIncrease |
            BentLineIncrease => 0,
            LinearDecrease |
            ExpDecrease => MAX_GAIN,
            _ => panic!("Don't initialise with state {:?}", self)
        }
    }
}