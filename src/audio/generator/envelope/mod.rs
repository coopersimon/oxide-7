// Envelope which alters the gain of the samples.
mod adsr;
mod gain;

use adsr::*;
use gain::*;

const GAIN_STEP: f32 = 1.0 / 64.0;      // Linear increase/decrease gain adjustment per step.
const BENT_STEP: f32 = 1.0 / 256.0;     // Bent line slow increase gain adjustment per step.
const EXP_STEP: f32 = 255.0 / 256.0;    // Exponential factor decrease gain adjustment per step.

const BENT_MAX: f32 = 3.0 / 4.0;        // Point at which bent line switches from fast to slow increase.
const EXP_MARGIN: f32 = 0.0001;         // Margin of error for exponential switching.

pub struct Envelope {
    sample_rate:    f64,

    adsr:           ADSRSettings,
    gain:           GainSettings,
    sustain:        f32,
    sustain_gap:    f32,

    state:          EnvelopeState,
    count:          f64,

    envelope:       f32,
}

impl Envelope {
    pub fn new(adsr: u16, gain: u8, sample_rate: f64) -> Self {
        let adsr_settings = ADSRSettings::from_bits_truncate(adsr);
        let gain_settings = GainSettings::from_bits_truncate(gain);

        let state = EnvelopeState::new(adsr_settings, gain_settings, sample_rate);
        let sustain = adsr_settings.sustain_level();
        let initial_gain = state.initial_gain();

        Envelope {
            sample_rate:    sample_rate,

            adsr:           adsr_settings,
            gain:           gain_settings,
            sustain:        sustain,
            sustain_gap:    1.0 - sustain,

            state:          state,
            count:          0.0,

            envelope:       initial_gain
        }
    }
}

impl Iterator for Envelope {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            // ADSR
            EnvelopeState::Attack(step_len) => {
                let out = self.envelope;
                if self.count >= step_len {
                    self.envelope += GAIN_STEP;
                }
                if self.envelope >= 1.0 {
                    self.state = EnvelopeState::Decay(self.adsr.decay(self.sample_rate));
                }
                Some(out)
            },
            EnvelopeState::Decay(step_len) => {
                let out = self.envelope;
                if self.count >= step_len {
                    let over_sustain = self.envelope - self.sustain;
                    let sustain_mul = over_sustain * EXP_STEP;
                    self.envelope = self.sustain + sustain_mul;
                }
                // TODO: check this margin
                if self.envelope <= self.sustain + EXP_MARGIN {
                    self.envelope = self.sustain;
                    self.state = self.adsr.sustain_release(self.sample_rate).map_or(
                        EnvelopeState::Static(self.sustain),
                        |sustain_release_step| EnvelopeState::Sustain(sustain_release_step)
                    );
                }
                Some(out)
            },
            EnvelopeState::Sustain(step_len) => {
                let out = self.envelope;
                if self.count >= step_len {
                    self.envelope *= EXP_STEP;
                }
                // TODO: check this margin
                if self.envelope <= EXP_MARGIN {
                    self.state = EnvelopeState::Off;
                }
                Some(out)
            },
            // Gain
            EnvelopeState::LinearIncrease(step_len) => {
                let out = self.envelope;
                if self.count >= step_len {
                    self.envelope += GAIN_STEP;
                }
                if self.envelope >= 1.0 {
                    self.state = EnvelopeState::Static(1.0);
                }
                Some(out)
            },
            EnvelopeState::BentLineIncrease(step_len) => {
                let out = self.envelope;
                if self.count >= step_len {
                    self.envelope += if self.envelope >= BENT_MAX {
                        BENT_STEP
                    } else {
                        GAIN_STEP
                    };
                }
                if self.envelope >= 1.0 {
                    self.state = EnvelopeState::Static(1.0);
                }
                Some(out)
            },
            EnvelopeState::LinearDecrease(step_len) => {
                let out = self.envelope;
                if self.count >= step_len {
                    self.envelope -= GAIN_STEP;
                }
                if self.envelope <= 0.0 {
                    self.state = EnvelopeState::Static(0.0);
                }
                Some(out)
            },
            EnvelopeState::ExpDecrease(step_len) => {
                let out = self.envelope;
                if self.count >= step_len {
                    self.envelope *= EXP_STEP;
                }
                // TODO: check this margin
                if self.envelope <= EXP_MARGIN {
                    self.state = EnvelopeState::Static(0.0);
                }
                Some(out)
            },

            // Static
            EnvelopeState::Static(val) => Some(val),
            EnvelopeState::Off => None,
        }
    }
}

// States along with the step time for each change.
// The associated values here are the step size; i.e. how many samples should be emitted before altering gain.
#[derive(Debug)]
enum EnvelopeState {
    // ADSR
    Attack(f64),  // Increase of 1/64 per step.
    Decay(f64),   // Dn = D(n-1) * (255/256) per step; D0 = 1.0 - sustainLevel; Gn = sustainLevel + Dn
    Sustain(f64), // Gn = G(n-1) * (255/256) per step, G0 = sustainLevel
    // GAIN
    LinearIncrease(f64),    // Increase of 1/64 per step.
    BentLineIncrease(f64),  // Increase of 1/64 until 3/4, then 1/256.
    LinearDecrease(f64),    // Decrease of 1/64 per step.
    ExpDecrease(f64),       // Gn = G(n-1) * (255/256) per step, G0 = 1.0.
    // STATIC
    Static(f32),    // Static gain or sustain. This assoc value is the LEVEL, not the step size.
    Off,            // Post-envelope.
}

impl EnvelopeState {
    #[inline]
    fn new(adsr: ADSRSettings, gain: GainSettings, sample_rate: f64) -> Self {
        if adsr.contains(ADSRSettings::ENABLE) {
            adsr.attack(sample_rate).map_or(
                EnvelopeState::Decay(adsr.decay(sample_rate)),
                |attack_step| EnvelopeState::Attack(attack_step)
            )
        } else {
            gain.state(sample_rate)
        }
    }

    #[inline]
    fn initial_gain(&self) -> f32 {
        use EnvelopeState::*;
        match self {
            Attack(_) |
            LinearIncrease(_) |
            BentLineIncrease(_) => 0.0,
            Decay(_) |
            LinearDecrease(_) |
            ExpDecrease(_) => 1.0,
            Static(v) => *v,
            _ => panic!("Don't initialise with state {:?}", self)
        }
    }
}