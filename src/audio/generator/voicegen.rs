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

    pitch:          u16,
    freq_step:      f64,
    freq_counter:   f64,

    envelope:       Envelope,

    enable:         bool,
    noise:          bool,
    pitch_mod:      bool,

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

            pitch:          0,
            freq_step:      0.0,
            freq_counter:   0.0,

            envelope:       Envelope::new(0, 0, sample_rate as f64),

            enable:         false,
            noise:          false,
            pitch_mod:      false,

            vol_left:       0.0,
            vol_right:      0.0,
        }
    }

    // Init sound from key on signal.
    pub fn key_on(&mut self, data: VoiceData) {
        self.sample = data.sample;
        self.s_loop = data.s_loop;
        self.source = SampleSource::Samp(0);

        self.pitch = data.regs.read_pitch();
        self.freq_step = self.sample_rate / freq_from_pitch(self.pitch);
        self.freq_counter = 0.0;

        self.envelope = Envelope::new(data.regs.read_adsr(), data.regs.read_gain(), self.sample_rate);

        self.enable = true;
        self.noise = data.regs.is_noise_enabled();    // TODO: maybe this should be set separately.
        self.pitch_mod = data.regs.is_pitch_mod_enabled();
        //println!("Key on pitch mod: {}", self.pitch_mod);

        self.vol_left = data.regs.read_left_vol();
        self.vol_right = data.regs.read_right_vol();
    }

    // Turn sound off from key off signal.
    pub fn key_off(&mut self) {
        if self.enable {
            self.envelope.off(0.008 * self.sample_rate);
        }
    }

    pub fn pitch_modulate(&mut self, p: i16) {
        if self.pitch_mod {
            let p_factor = (p >> 4) + 0x400;
            let new_pitch = ((self.pitch as u32) * (p_factor as u32)) >> 10;
            self.freq_step = freq_from_pitch((new_pitch as u16) & 0x3FFF);
            //self.freq_counter = 0.0;
        }
    }

    pub fn next(&mut self) -> i16 {
        if self.enable {
            if !self.noise {
                let sample = self.calc_interpolated_sample();
                self.step_freq();
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
    fn calc_interpolated_sample(&self) -> f32 {
        let s_a = match self.source {
            SampleSource::Samp(i) => self.sample[i],
            SampleSource::Loop(i) => self.s_loop[i],
        };
        let s_b = match self.next_sample() {
            Some(SampleSource::Samp(i)) => self.sample[i],
            Some(SampleSource::Loop(i)) => self.s_loop[i],
            None => 0.0
        };

        let frac = (self.freq_counter / self.freq_step) as f32;
        (s_a * (1.0 - frac)) + (s_b * frac)
    }

    fn step_freq(&mut self) {
        self.freq_counter += 1.0;
        while self.freq_counter >= self.freq_step {
            match self.next_sample() {
                Some(s) => self.source = s,
                None => self.enable = false,
            }

            self.freq_counter -= self.freq_step;
        }
    }

    fn next_sample(&self) -> Option<SampleSource> {
        match self.source {
            SampleSource::Samp(i) => {
                let new_i = i + 1;
                if new_i >= self.sample.len() {
                    if self.should_loop() {
                        Some(SampleSource::Loop(0))
                    } else {
                        None
                    }
                } else {
                    Some(SampleSource::Samp(new_i))
                }
            },
            SampleSource::Loop(i) => {
                let new_i = i + 1;
                if new_i >= self.s_loop.len() {
                    Some(SampleSource::Loop(0))
                } else {
                    Some(SampleSource::Loop(new_i))
                }
            }
        }
    }

    fn should_loop(&self) -> bool {
        self.s_loop.len() > 0
    }
}

fn freq_from_pitch(pitch: u16) -> f64 {
    const AUDIO_FREQ: f64 = 32_000.0;
    const PITCH_FACTOR: usize = bit!(12, u16) as usize;
    const PITCH_COEF: f64 = AUDIO_FREQ / (PITCH_FACTOR as f64);

    (pitch as f64) * PITCH_COEF
}