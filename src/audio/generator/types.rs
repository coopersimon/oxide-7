// Types for use with generating audio.
use super::super::dsp::Voice;

#[derive(Clone)]
pub struct VoiceData {
    pub regs:   Box<Voice>, // Register values
    pub sample: Box<[f32]>, // Sample data
    pub s_loop: Box<[f32]>, // Sample loop data
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