// Generates audio samples for a single audio voice.
use super::super::dsp::Voice;
use super::types::VoiceData;

#[derive(Default)]
pub struct VoiceGen {

}

impl VoiceGen {
    pub fn new() -> Self {
        VoiceGen {

        }
    }

    // Init sound from key on signal.
    pub fn key_on(&mut self, data: &VoiceData) {
        
    }

    // Turn sound off from key off signal.
    pub fn key_off(&mut self) {

    }

    // Generates the sound and inserts it into the buffer at the fractions provided.
    pub fn generate_signal(&mut self, buffer: &mut [i16], start_time: f32, end_time: f32) {

    }
}