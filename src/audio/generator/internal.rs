// Receives data from the DSP and outputs samples.

use crossbeam_channel::Receiver;

use super::{
    types::*,
    voicegen::VoiceGen,
    super::dsp::Voice
};

use std::collections::VecDeque;

// Receives updates from the AudioDevice, and processes and generates signals.
pub struct InternalAudioGenerator {
    receiver:   Receiver<AudioData>,

    // Data lists for each note
    voice_data:   [VecDeque<(Option<Voice>, f32)>; 8],

    // Signal generators
    voice_generators:   [VoiceGen; 8],

    // Control values
    // ...?

    // Raw channel buffers
    buffers:    AudioBuffers,
}

impl InternalAudioGenerator {
    pub fn new(recv: Receiver<AudioData>, sample_rate: usize) -> Self {
        InternalAudioGenerator {
            receiver:   recv,

            voice_data:         Default::default(),

            voice_generators:   Default::default(),

            buffers:    AudioBuffers::new(sample_rate / 60),
        }
    }

    // Generator function that produces the next two samples (left & right channel)
    pub fn process_frame(&mut self) -> AudioFrame {
        match self.buffers.next() {
            Some(vals) => self.mix_output(vals),
            None => {
                // Fetch updates - keep waiting until we get frame.
                loop {
                    let data = self.receiver.recv().unwrap();
                    match data {
                        AudioData::VoiceKeyOn{
                            num, data, time
                        } => self.voice_data[num].push_back((Some(data), time)),
                        AudioData::VoiceKeyOff{
                            num, time
                        } => self.voice_data[num].push_back((None, time)),
                        AudioData::DSP(/*DSPRegs*/) => {
                            // Set DSP stuff
                            break;
                        },
                        AudioData::Frame => break,
                    }
                }

                // Generate signals for each buffer
                for ((gen, data), buffer) in self.voice_generators.iter_mut().zip(self.voice_data.iter_mut()).zip(self.buffers.voices.iter_mut()) {
                    process_command_buffer(gen, data, buffer);
                }

                // Mix first samples of new data.
                match self.buffers.next() {
                    Some(vals) => self.mix_output(vals),
                    None => panic!("Can't find any audio."),
                }
            },
        }
    }
}

impl InternalAudioGenerator {
    #[inline]
    fn mix_output(&mut self, vals: [i16; 8]) -> AudioFrame {
        [0.0, 0.0]
    }
}

#[inline]
fn process_command_buffer(gen: &mut VoiceGen, data: &mut VecDeque<(Option<Voice>, f32)>, buffer: &mut [i16]) {
    // First note:
    let end_time = if data.len() > 0 {data[0].1} else {1.0};
    gen.generate_signal(buffer, 0.0, end_time);

    for i in 0..data.len() {
        match data[i].0 {
            Some(data) => gen.key_on(&data),
            None => gen.key_off()
        }

        let start_time = data[i].1;
        let end_time = if i + 1 < data.len() {data[i + 1].1} else {1.0};

        gen.generate_signal(buffer, start_time, end_time);
    }

    data.clear();
}

struct AudioBuffers {
    voices:     [Vec<i16>; 8],

    size:       usize,
    i:          usize,
}

impl AudioBuffers {
    fn new(buffer_size: usize) -> Self {
        AudioBuffers {
            voices:     [
                vec![0; buffer_size],
                vec![0; buffer_size],
                vec![0; buffer_size],
                vec![0; buffer_size],
                vec![0; buffer_size],
                vec![0; buffer_size],
                vec![0; buffer_size],
                vec![0; buffer_size],
            ],

            size:       buffer_size,
            i:          0,
        }
    }
}

impl Iterator for AudioBuffers {
    type Item = [i16; 8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.size {
            self.i = 0;
            None
        } else {
            let ret = [
                self.voices[0][self.i],
                self.voices[1][self.i],
                self.voices[2][self.i],
                self.voices[3][self.i],
                self.voices[4][self.i],
                self.voices[5][self.i],
                self.voices[6][self.i],
                self.voices[7][self.i]
            ];
            self.i += 1;
            Some(ret)
        }
    }
}