// Generates audio samples for a single audio voice.
use super::adsr::ADSRSettings;
use super::types::VoiceData;

enum Status {
    On,
    Fade(u8),
    Off
}

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
    status:         Status
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

            envelope:       Envelope::new(0, 0),

            noise:          false,
            status:         Status::Off
        }
    }

    // Init sound from key on signal.
    pub fn key_on(&mut self, data: VoiceData) {
        self.sample = data.sample;
        self.s_loop = data.s_loop;
        self.source = SampleSource::Samp(0);

        self.freq_step = self.sample_rate / data.regs.freq();
        self.freq_counter = 0.0;

        self.envelope = Envelope::new(data.regs.read_adsr(), data.regs.read_gain());

        self.noise = data.regs.is_noise_enabled();    // TODO: maybe this should be set separately.

        self.status = Status::On;
    }

    // Turn sound off from key off signal.
    pub fn key_off(&mut self) {
        self.status = Status::Fade(255);
    }

    // Generates the sound and inserts it into the buffer at the fractions provided.
    pub fn generate_signal(&mut self, buffer: &mut [i16], start_time: f32, end_time: f32) {
        let take = (buffer.len() as f32 * end_time) as usize;
        let skip = (buffer.len() as f32 * start_time) as usize;

        for i in buffer.iter_mut().take(take).skip(skip) {
            if !self.noise {
                let sample = match self.source {
                    SampleSource::Samp(i) => self.sample[i],
                    SampleSource::Loop(i) => self.s_loop[i],
                };
                
            } else {
                // TODO
                *i = 0;
            }
        }
    }
}

// ADSR

struct Envelope {
    adsr:   ADSRSettings,
    gain:   u8,
}

impl Envelope {
    fn new(adsr: u16, gain: u8) -> Self {
        Envelope {
            adsr:   ADSRSettings::from_bits_truncate(adsr),
            gain:   gain,
        }
    }
}