// Generates audio samples for a single audio voice.
use super::envelope::*;
use super::types::VoiceData;

enum SampleSource {
    Samp(usize),
    Loop(usize)
}

pub struct VoiceGen {
    sample_rate:    f64,

    sample:         Box<[f32]>, // Current sample sound.
    s_loop:         Box<[f32]>, // Current sample loop sound.
    source:         SampleSource,

    freq_step:      f64,
    freq_counter:   f64,

    envelope:       Envelope,

    noise:          bool,
    enable:         bool,

    vol_left:       f32,    // 0 -> 1
    vol_right:      f32,
}

impl VoiceGen {
    pub fn new(sample_rate: usize) -> Self {
        VoiceGen {
            sample_rate:    sample_rate as f64,

            sample:         Box::new([]),
            s_loop:         Box::new([]),
            source:         SampleSource::Samp(0),

            freq_step:      0.0,
            freq_counter:   0.0,

            envelope:       Envelope::new(0, 0, sample_rate as f64),

            noise:          false,
            enable:         false,

            vol_left:       0.0,
            vol_right:      0.0,
        }
    }

    // Init sound from key on signal.
    pub fn key_on(&mut self, data: VoiceData) {
        self.sample = data.sample;
        self.s_loop = data.s_loop;
        self.source = SampleSource::Samp(0);

        let freq = data.regs.freq();
        self.freq_step = self.sample_rate / freq;
        self.freq_counter = 0.0;

        self.envelope = Envelope::new(data.regs.read_adsr(), data.regs.read_gain(), self.sample_rate);

        self.enable = true;
        self.noise = data.regs.is_noise_enabled();    // TODO: maybe this should be set separately.

        self.vol_left = data.regs.read_left_vol();
        self.vol_right = data.regs.read_right_vol();
    }

    // Turn sound off from key off signal.
    pub fn key_off(&mut self) {
        if self.enable {
            self.envelope.off(0.008 * self.sample_rate);
        }
    }

    pub fn next(&mut self, p_mod: i16) -> i16 {
        if self.enable {
            if !self.noise {
                let sample = match self.source {
                    SampleSource::Samp(i) => self.sample[i],
                    SampleSource::Loop(i) => self.s_loop[i],
                };
                self.step_freq();
                // TODO: pitch mod
                match self.envelope.next() {
                    Some(gain) => (gain * sample) as i16,
                    None => {
                        self.enable = false;
                        0
                    }
                }
            } else {
                // TODO
                0
            }
        } else {
            0
        }
    }

    pub fn get_vol_left(&self) -> f32 {
        self.vol_left
    }

    pub fn get_vol_right(&self) -> f32 {
        self.vol_right
    }
}

impl VoiceGen {
    fn step_freq(&mut self) {
        self.freq_counter += 1.0;
        while self.freq_counter >= self.freq_step {
            match self.source {
                SampleSource::Samp(i) => {
                    let new_i = i + 1;
                    if new_i >= self.sample.len() {
                        if self.should_loop() {
                            self.source = SampleSource::Loop(0);
                        } else {
                            self.enable = false;
                        }
                    } else {
                        self.source = SampleSource::Samp(new_i);
                    }
                },
                SampleSource::Loop(i) => {
                    let new_i = i + 1;
                    if new_i >= self.s_loop.len() {
                        self.source = SampleSource::Loop(0);
                    } else {
                        self.source = SampleSource::Loop(new_i);
                    }
                }
            }

            self.freq_counter -= self.freq_step;
        }
    }

    fn should_loop(&self) -> bool {
        self.s_loop.len() > 0
    }
}
