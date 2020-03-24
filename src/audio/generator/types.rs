// Types for use with generating audio.
use super::super::dsp::Voice;

#[derive(Clone, Debug)]
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

    // DSP things
    Mute(bool),
    DSPVolLeft(f32),
    DSPVolRight(f32),

    Frame,
}

pub type AudioFrame = [f32; 2];