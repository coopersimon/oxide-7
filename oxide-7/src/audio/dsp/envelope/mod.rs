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
    sustain:        i16,

    state:          EnvelopeState,
    count:          usize,

    gain:           i16,
}

impl Envelope {
    pub fn new(adsr: u16, gain: u8) -> Self {
        let adsr_settings = ADSRSettings::from_bits_truncate(adsr);
        let gain_settings = GainSettings::from_bits_truncate(gain);

        let state = EnvelopeState::new(adsr_settings, gain_settings);
        let sustain = adsr_settings.sustain_level();
        let initial_gain = state.initial_gain();

        Envelope {
            adsr:           adsr_settings,
            sustain:        sustain,

            state:          state,
            count:          0,

            gain:           initial_gain
        }
    }

    pub fn off(&mut self) {
        self.count = 0;
        self.state = EnvelopeState::Fade;
    }

    pub fn muted(&self) -> bool {
        self.state == EnvelopeState::Fade && self.gain <= 0
    }
}

impl Iterator for Envelope {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            // ADSR
            EnvelopeState::Attack(step_len) => {
                let out = self.gain;
                self.count += 1;
                if self.count >= step_len {
                    self.gain += GAIN_STEP;
                    self.count = 0;
                }
                if self.gain >= MAX_ATTACK {
                    self.gain = MAX_ATTACK;
                    self.state = EnvelopeState::Decay(self.adsr.decay());
                }
                Some(out)
            },
            EnvelopeState::Decay(step_len) => {
                let out = self.gain;
                self.count += 1;
                if self.count >= step_len {
                    let decay_factor = ((self.gain - 1) >> 8) + 1;
                    self.gain = self.gain - decay_factor;
                    self.count = 0;
                }
                if self.gain <= self.sustain {
                    self.state = self.adsr.sustain_release().map_or(
                        EnvelopeState::Static(self.sustain),
                        |sustain_release_step| EnvelopeState::Sustain(sustain_release_step)
                    );
                }
                Some(out)
            },
            EnvelopeState::Sustain(step_len) => {
                let out = self.gain;
                self.count += 1;
                if self.count >= step_len {
                    let decay_factor = ((self.gain - 1) >> 8) + 1;
                    self.gain = self.gain - decay_factor;
                    self.count = 0;
                }
                if self.gain <= 0 {
                    self.gain = 0;
                    self.state = EnvelopeState::Static(0);
                }
                Some(out)
            },
            // Gain
            EnvelopeState::LinearIncrease(step_len) => {
                let out = self.gain;
                self.count += 1;
                if self.count >= step_len {
                    self.gain += GAIN_STEP;
                    self.count = 0;
                }
                if self.gain >= MAX_GAIN {
                    self.gain = MAX_GAIN;
                    self.state = EnvelopeState::Static(MAX_GAIN);
                }
                Some(out)
            },
            EnvelopeState::BentLineIncrease(step_len) => {
                let out = self.gain;
                self.count += 1;
                if self.count >= step_len {
                    self.gain += if self.gain >= BENT_MAX {
                        BENT_STEP
                    } else {
                        GAIN_STEP
                    };
                    self.count = 0;
                }
                if self.gain >= MAX_GAIN {
                    self.gain = MAX_GAIN;
                    self.state = EnvelopeState::Static(MAX_GAIN);
                }
                Some(out)
            },
            EnvelopeState::LinearDecrease(step_len) => {
                let out = self.gain;
                self.count += 1;
                if self.count >= step_len {
                    self.gain -= GAIN_STEP;
                    self.count = 0;
                }
                if self.gain <= 0 {
                    self.gain = 0;
                    self.state = EnvelopeState::Static(0);
                }
                Some(out)
            },
            EnvelopeState::ExpDecrease(step_len) => {
                let out = self.gain;
                self.count += 1;
                if self.count >= step_len {
                    let decay_factor = ((self.gain - 1) >> 8) + 1;
                    self.gain = self.gain - decay_factor;
                    self.count = 0;
                }
                if self.gain <= 0 {
                    self.gain = 0;
                    self.state = EnvelopeState::Static(0);
                }
                Some(out)
            },

            // Fade
            EnvelopeState::Fade if self.gain > 0 => {
                let out = self.gain;
                self.gain -= BENT_STEP;
                Some(out)
            },
            EnvelopeState::Fade => None,    // if gain <= 0

            // Static
            EnvelopeState::Static(val) => Some(val),
        }
    }
}

// States along with the step time for each change.
// The associated values here are the step size; i.e. how many samples should be emitted before altering gain.
#[derive(Debug, PartialEq)]
pub enum EnvelopeState {
    // ADSR
    Attack(usize),  // Increase of 1/64 per step.
    Decay(usize),   // Dn = D(n-1) * (255/256) per step; D0 = 1.0 - sustainLevel; Gn = sustainLevel + Dn
    Sustain(usize), // Gn = G(n-1) * (255/256) per step, G0 = sustainLevel
    // GAIN
    LinearIncrease(usize),    // Increase of 1/64 per step.
    BentLineIncrease(usize),  // Increase of 1/64 until 3/4, then 1/256.
    LinearDecrease(usize),    // Decrease of 1/64 per step.
    ExpDecrease(usize),       // Gn = G(n-1) * (255/256) per step, G0 = 1.0.
    // STATIC
    Fade,           // On key off. Decrease of 1/256 per step.
    Static(i16),    // Static gain or sustain. This assoc value is the LEVEL, not the step size.
}

impl EnvelopeState {
    #[inline]
    fn new(adsr: ADSRSettings, gain: GainSettings) -> Self {
        if adsr.contains(ADSRSettings::ENABLE) {
            adsr.attack().map_or(
                EnvelopeState::Decay(adsr.decay()),
                |attack_step| EnvelopeState::Attack(attack_step)
            )
        } else {
            gain.state()
        }
    }

    #[inline]
    fn initial_gain(&self) -> i16 {
        use EnvelopeState::*;
        match self {
            Attack(_) |
            LinearIncrease(_) |
            BentLineIncrease(_) => 0,
            Decay(_) |
            LinearDecrease(_) |
            ExpDecrease(_) => MAX_GAIN,
            Static(v) => *v,
            _ => panic!("Don't initialise with state {:?}", self)
        }
    }
}