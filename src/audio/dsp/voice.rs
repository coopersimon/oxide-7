// A single audio channel

const AUDIO_FREQ: f64 = 32_000.0;

const PITCH_MASK: u16 = 0x3FFF;
const PITCH_FACTOR: usize = bit!(12, u16) as usize;
const PITCH_COEF: f64 = AUDIO_FREQ / (PITCH_FACTOR as f64);

#[derive(Clone, Copy)]
pub struct Voice {
    left_vol:   u8, // Signed magnitude representation
    right_vol:  u8,

    pitch:      u16,

    src_num:    u8, // Lower byte of memory addr to use for sample.

    adsr:       u16,
    gain:       u8,
    fir_coef:   u8,

    noise:      bool,   // Should this voice generate noise

    // Read
    envx:       u8,
    outx:       u8,
}

impl Voice {
    pub fn new() -> Self {
        Voice {
            left_vol:   0,
            right_vol:  0,

            pitch:      0,

            src_num:    0,

            adsr:       0,
            gain:       0,
            fir_coef:   0,

            noise:      false,

            envx:       0,
            outx:       0,
        }
    }

    // Uses a 4-bit address to index registers.
    pub fn read(&self, addr: u8) -> u8 {
        match addr & 0xF {
            0x0 => self.left_vol,
            0x1 => self.right_vol,
            0x2 => lo!(self.pitch),
            0x3 => hi!(self.pitch),
            0x4 => self.src_num,
            0x5 => lo!(self.adsr),
            0x6 => hi!(self.adsr),
            0x7 => self.gain,
            0x8 => self.envx,
            0x9 => self.outx,
            0xF => self.fir_coef,
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr & 0xF {
            0x0 => self.left_vol = data,
            0x1 => self.right_vol = data,
            0x2 => self.pitch = set_lo!(self.pitch, data),
            0x3 => self.pitch = set_hi!(self.pitch, data),
            0x4 => self.src_num = data,
            0x5 => self.adsr = set_lo!(self.adsr, data),
            0x6 => self.adsr = set_hi!(self.adsr, data),
            0x7 => self.gain = data,
            0xF => self.fir_coef = data,
            _ => {},
        }
    }
}

// DSP
impl Voice {
    // Index into source directory.
    pub fn dir_index(&self) -> u16 {
        (self.src_num as u16) * 4
    }

    pub fn enable_noise(&mut self, enable: bool) {
        self.noise = enable;
    }

    pub fn is_noise_enabled(&self) -> bool {
        self.noise
    }

    pub fn freq(&self) -> f64 {
        let pitch = self.pitch & PITCH_MASK;
        (pitch as f64) * PITCH_COEF
    }

    pub fn read_adsr(&self) -> u16 {
        self.adsr
    }

    pub fn read_gain(&self) -> u8 {
        self.gain
    }

    pub fn read_left_vol(&self) -> f32 {
        ((self.left_vol as i8) as f32) / 128.0
    }

    pub fn read_right_vol(&self) -> f32 {
        ((self.right_vol as i8) as f32) / 128.0
    }
}