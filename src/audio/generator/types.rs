// Types for use with generating audio.
use super::super::dsp::Voice;

pub enum AudioData {
    VoiceKeyOn{
        num:    usize,
        data:   Voice,
        time:   f32
    },
    VoiceKeyOff{
        num:    usize,
        time:   f32
    },
    DSP(/*DSPRegs*/),
    Frame,
}

pub type AudioFrame = [f32; 2];