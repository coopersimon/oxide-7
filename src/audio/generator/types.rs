// Types for use with generating audio.
use super::super::dsp::Voice;

pub struct VoiceData {
    pub regs:   Box<Voice>, // Register values
    pub sample: Box<[i16]>, // Sample data
    pub s_loop: Box<[i16]>, // Sample loop data
}

pub enum AudioData {
    VoiceKeyOn{
        num:    usize,      // Voice number
        data:   VoiceData,
        time:   f32         // Time in video frame that key on was triggered
    },
    VoiceKeyOff{
        num:    usize,
        time:   f32
    },
    DSP(/*DSPRegs*/),
    Frame,
}

pub type AudioFrame = [f32; 2];