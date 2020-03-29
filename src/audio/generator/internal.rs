// Receives data from the DSP and outputs samples.

use crossbeam_channel::Receiver;

use super::{
    types::*,
    voicegen::VoiceGen
};

use std::collections::VecDeque;

// Receives updates from the AudioDevice, and processes and generates signals.
pub struct InternalAudioGenerator {
    receiver:       Receiver<AudioData>,
    process_step:   f32,
    buffer:         AudioBuffer,

    // Data lists for each note
    voice_data:     [VecDeque<(Option<VoiceData>, usize)>; 8],

    // Signal generators
    voice_generators:   [VoiceGen; 8],

    // Control values
    mute:       bool,
    vol_left:   f32,
    vol_right:  f32,

    // Previous buffer values
    previous: [i16; 8]
}

impl InternalAudioGenerator {
    pub fn new(recv: Receiver<AudioData>, sample_rate: usize) -> Self {
        let process_step = sample_rate / 60;
        InternalAudioGenerator {
            receiver:       recv,
            process_step:   process_step as f32,
            buffer:         AudioBuffer::new(32_000, sample_rate),

            voice_data:     Default::default(),

            voice_generators:   [
                VoiceGen::new(sample_rate),
                VoiceGen::new(sample_rate),
                VoiceGen::new(sample_rate),
                VoiceGen::new(sample_rate),
                VoiceGen::new(sample_rate),
                VoiceGen::new(sample_rate),
                VoiceGen::new(sample_rate),
                VoiceGen::new(sample_rate),
            ],

            mute:       true,
            vol_left:   0.0,
            vol_right:  0.0,

            previous:   [0; 8],
        }
    }

    // Generator function that produces the next two samples (left & right channel)
    pub fn process_frame(&mut self) -> AudioFrame {
        match self.buffer.next() {
            Some(frame) => frame,
            None => {
                // Fetch updates - keep waiting until we get frame.
                loop {
                    let data = self.receiver.recv().unwrap();
                    match data {
                        AudioData::VoiceKeyOn{
                            num, data, time
                        } => self.voice_data[num].push_back((Some(data), (time * self.process_step) as usize)),
                        AudioData::VoiceKeyOff{
                            num, time
                        } => self.voice_data[num].push_back((None, (time * self.process_step) as usize)),
                        AudioData::Mute(m) => self.mute = m,
                        AudioData::DSPVolLeft(v) => self.vol_left = v,  // TODO: make this more fine-grained
                        AudioData::DSPVolRight(v) => self.vol_right = v,
                        AudioData::Frame => break,
                    }
                }

                // Generate a full buffer of samples.
                self.generate_and_mix();

                // Mix first samples of new data.
                match self.buffer.next() {
                    Some(frame) => frame,
                    None => panic!("Can't find any audio."),
                }
            },
        }
    }
}

const CHAN_DIV_FACTOR: f32 = 1.0 / (8.0 * 32768.0);

impl InternalAudioGenerator {

    // Process data for the (video) frame.
    fn generate_and_mix(&mut self) {
        // Get change points for each channel.
        let mut next_gen = self.voice_data.iter().map(|queue| {
            queue.front().and_then(|(_, t)| Some(*t)).unwrap_or(self.process_step as usize)
        }).collect::<Vec<_>>();

        let mut current = vec![0; 8];

        for (i, d) in self.buffer.in_buffer.iter_mut().enumerate() {
            d[0] = 0.0;
            d[1] = 0.0;

            if self.mute {
                continue;
            }
            for v in 0..8 {
                // Update generator sound if necessary.
                if i >= next_gen[v] {
                    let (voice_data, _) = self.voice_data[v].pop_front().expect("Popped empty voice data buffer!");
                    match voice_data {
                        Some(data) => self.voice_generators[v].key_on(data, v),
                        None => self.voice_generators[v].key_off()
                    }
                    next_gen[v] = self.voice_data[v].front().map(|(_, t)| *t).unwrap_or(self.process_step as usize);
                }

                // Generate and mix sample.
                if v > 0 {
                    self.voice_generators[v].pitch_modulate(self.previous[v - 1]);
                }
                let outx = self.voice_generators[v].next();
                current[v] = outx;
                let samp = (outx as f32) * CHAN_DIV_FACTOR;
                d[0] += samp * self.voice_generators[v].get_vol_left();
                d[1] += samp * self.voice_generators[v].get_vol_right();
            }

            for v in 0..8 {
                self.previous[v] = current[v];
            }

            d[0] *= self.vol_left;
            d[1] *= self.vol_right;
        }

        self.buffer.fill_output_buffer();
    }
}

use samplerate::{Samplerate, ConverterType};

// Gets generated samples and resamples them to the output rate.
struct AudioBuffer {
    resampler:  Samplerate,

    in_buffer:  Vec<[f32; 2]>,

    out_buffer: Option<Vec<f32>>,
    i:          usize,
}

impl AudioBuffer {
    fn new(sample_rate_in: usize, sample_rate_out: usize) -> Self {
        let resampler = Samplerate::new(ConverterType::SincBestQuality, sample_rate_in as u32, sample_rate_out as u32, 2)
            .expect("Couldn't make resampler");

        AudioBuffer {
            resampler:  resampler,
            in_buffer:  vec![[0.0, 0.0]; sample_rate_in / 60],
            out_buffer: None,//vec![[0.0, 0.0]; sample_rate_out / 60],
            i:          0,
        }
    }

    // Call this after generating a frame's worth of input samples.
    fn fill_output_buffer(&mut self) {
        self.out_buffer = Some(self.resampler.process(
            &self.in_buffer
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>()
            ).expect("Error occurred when resampling."));
    }
}

impl Iterator for AudioBuffer {
    type Item = [f32; 2];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(buffer) = self.out_buffer {
            if self.i >= buffer.len() {
                self.i = 0;
                None
            } else {
                let ret = [buffer[self.i], buffer[self.i + 1]];
                self.i += 2;
                Some(ret)
            }
        } else {
            None
        }
    }
}