// Various constants used in calculating audio output.

const AUDIO_FREQ: f64 = 32_000.0;

const PITCH_FACTOR: usize = bit!(12) as usize;
const PITCH_COEF: f64 = AUDIO_FREQ / (PITCH_FACTOR as f64);