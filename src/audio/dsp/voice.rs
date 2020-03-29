// A single audio channel

use super::envelope::Envelope;

#[derive(Clone, Copy)]
// These indicate the current BRR sample number.
enum SampleSource {
    Samp(usize),
    Loop(usize)
}

const PITCH_MASK: u16 = 0x3FFF;

pub struct Voice {
    left_vol:   u8, // Signed magnitude representation
    right_vol:  u8,

    pitch:      u16,

    src_num:    u8, // Lower byte of memory addr to use for sample.

    adsr:       u16,
    gain:       u8,
    fir_coef:   u8,

    noise:      bool,   // Should this voice generate noise
    pitch_mod:  bool,

    // Read
    envx:       u8,
    outx:       u8,

    // Internal: sample generation
    // TODO: separate this?
    current_s:      Option<SampleSource>,
    sample:         Box<[i16]>, // Sample data
    s_loop:         Box<[i16]>, // Sample loop data
    envelope:       Envelope,
    freq_counter:   u16,
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
            pitch_mod:  false,

            envx:       0,
            outx:       0,

            current_s:      None,
            sample:         Box::new([]),
            s_loop:         Box::new([]),
            envelope:       Envelope::new(0, 0),
            freq_counter:   0,
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
            0x3 => self.pitch = set_hi!(self.pitch, data & 0x3F),
            0x4 => self.src_num = data,
            0x5 => self.adsr = set_lo!(self.adsr, data),
            0x6 => self.adsr = set_hi!(self.adsr, data),
            0x7 => self.gain = data,
            0xF => self.fir_coef = data,
            _ => {},
        }
    }

    // Key on / off
    pub fn key_on(&mut self, sample: Box<[i16]>, s_loop: Box<[i16]>) {
        self.sample = sample;
        self.s_loop = s_loop;
        self.current_s = Some(SampleSource::Samp(0));
        self.envelope = Envelope::new(self.adsr, self.gain);
        self.freq_counter = 0;
    }

    pub fn key_off(&mut self) {
        self.envelope.off();
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

    pub fn enable_pitch_mod(&mut self, enable: bool) {
        self.pitch_mod = enable;
    }

    // Is this channel currently keyed on?
    pub fn is_on(&self) -> bool {
        self.current_s.is_some()
    }

    /*pub fn read_pitch(&self) -> u16 {
        self.pitch & PITCH_MASK
    }

    pub fn read_adsr(&self) -> u16 {
        self.adsr
    }

    pub fn read_gain(&self) -> u8 {
        self.gain
    }*/

    pub fn read_left_vol(&self) -> f32 {
        ((self.left_vol as i8) as f32) / 128.0
    }

    pub fn read_right_vol(&self) -> f32 {
        ((self.right_vol as i8) as f32) / 128.0
    }
}

// Generator
impl Iterator for Voice {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(s) = self.current_s {
            let sample = self.generate_sample();
            self.freq_step(s);
            self.apply_envelope(sample)
        } else {
            None
        }
    }
}

// Clamp val between min and max.
macro_rules! clamp {
    ($val:expr, $min:expr, $max:expr) => {
        std::cmp::min($max, std::cmp::max($min, $val))
    };
}

// Internal: sample gen
impl Voice {
    // Mix sample using gaussian filter.
    fn generate_sample(&self) -> i16 {
        const MIN: i32 = std::i16::MIN as i32;
        const MAX: i32 = std::i16::MAX as i32;

        let gauss_index = ((self.freq_counter >> 4) & 0xFF) as usize;
        let samples = self.get_samples();
        let mut out = (samples[3] * GAUSS_TABLE[0xFF - gauss_index]) >> 10;
        out += (samples[2] * GAUSS_TABLE[0x1FF - gauss_index]) >> 10;
        out += (samples[1] * GAUSS_TABLE[0x100 + gauss_index]) >> 10;
        out += (samples[0] * GAUSS_TABLE[gauss_index]) >> 10;
        //let prev_1 = prev_2 + ();
        //let prev_0 = prev_1 + ();
        (clamp!(out, MIN, MAX) >> 1) as i16
    }

    // Get the current sample, and the previous 3.
    fn get_samples(&self) -> [i32; 4] {
        let s = self.current_s.expect("Get samples called on 'off' voice.");
        match s {
            SampleSource::Samp(i) => {
                let current = self.sample[i] as i32;
                let prev_0 = if i > 0 { self.sample[i - 1] } else { 0 } as i32;
                let prev_1 = if i > 1 { self.sample[i - 2] } else { 0 } as i32;
                let prev_2 = if i > 2 { self.sample[i - 3] } else { 0 } as i32;
                [current, prev_0, prev_1, prev_2]
            },
            SampleSource::Loop(i) => {
                let max_idx = self.s_loop.len() - 1;
                let current = self.s_loop[i] as i32;
                let prev_0 = if i > 0 { self.s_loop[i - 1] } else { self.s_loop[max_idx] } as i32;
                let prev_1 = if i > 1 { self.s_loop[i - 2] } else { self.s_loop[max_idx + i - 1] } as i32;
                let prev_2 = if i > 2 { self.s_loop[i - 3] } else { self.s_loop[max_idx + i - 2] } as i32;
                [current, prev_0, prev_1, prev_2]
            }
        }
    }

    // Step after outputting each sample.
    fn freq_step(&mut self, old_s: SampleSource) {
        let counter = (self.freq_counter as u32) + (self.pitch as u32);
        // Sample index is the sample number in the current or next BRR block.
        let sample_index = ((counter >> 12) as usize) & 0x1F;
        let sample_number = sample_index & 0xF;
        self.current_s = match old_s {
            SampleSource::Samp(i) => {
                let block_number = (i >> 4) + (sample_index >> 4);
                let block_offset = block_number << 4;
                if block_offset >= self.sample.len() {   // If sample has ended...
                    if self.s_loop.is_empty() {
                        None    // No loop: end playback.
                    } else {
                        Some(SampleSource::Loop(sample_number))
                    }
                } else {
                    let index = block_offset + sample_number;
                    Some(SampleSource::Samp(index))
                }
            },
            SampleSource::Loop(i) => {
                let block_number = (i >> 4) + (sample_index >> 4);
                let block_offset = block_number << 4;
                let index = block_offset + sample_number;
                Some(SampleSource::Loop(index % self.s_loop.len()))
            }
        };
        self.freq_counter = counter as u16;
    }

    // Check that the envelope indicates the sample is "on", and multiply the sample by the envelope.
    fn apply_envelope(&mut self, sample: i16) -> Option<i16> {
        const ENVX_SHIFT: usize = 4;
        const OUTX_SHIFT: usize = 7;

        if let Some(e) = self.envelope.next() {
            self.envx = ((e >> ENVX_SHIFT) & 0x7F) as u8;
            let s32 = sample as i32;
            let e32 = e as i32;
            let env_samp = ((s32 * e32) >> 11) as i16;
            self.outx = ((env_samp >> OUTX_SHIFT) & 0xFF) as u8;
            Some(env_samp)
        } else {
            self.current_s = None;
            self.envx = 0;
            self.outx = 0;
            None
        }
    }
}

// Gaussian interpolation coefficients
const GAUSS_TABLE: [i32; 512] = [
    0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000,
    0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x001, 0x002, 0x002, 0x002, 0x002, 0x002,
    0x002, 0x002, 0x003, 0x003, 0x003, 0x003, 0x003, 0x004, 0x004, 0x004, 0x004, 0x004, 0x005, 0x005, 0x005, 0x005,
    0x006, 0x006, 0x006, 0x006, 0x007, 0x007, 0x007, 0x008, 0x008, 0x008, 0x009, 0x009, 0x009, 0x00A, 0x00A, 0x00A,
    0x00B, 0x00B, 0x00B, 0x00C, 0x00C, 0x00D, 0x00D, 0x00E, 0x00E, 0x00F, 0x00F, 0x00F, 0x010, 0x010, 0x011, 0x011,
    0x012, 0x013, 0x013, 0x014, 0x014, 0x015, 0x015, 0x016, 0x017, 0x017, 0x018, 0x018, 0x019, 0x01A, 0x01B, 0x01B,
    0x01C, 0x01D, 0x01D, 0x01E, 0x01F, 0x020, 0x020, 0x021, 0x022, 0x023, 0x024, 0x024, 0x025, 0x026, 0x027, 0x028,
    0x029, 0x02A, 0x02B, 0x02C, 0x02D, 0x02E, 0x02F, 0x030, 0x031, 0x032, 0x033, 0x034, 0x035, 0x036, 0x037, 0x038,
    0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x040, 0x041, 0x042, 0x043, 0x045, 0x046, 0x047, 0x049, 0x04A, 0x04C, 0x04D,
    0x04E, 0x050, 0x051, 0x053, 0x054, 0x056, 0x057, 0x059, 0x05A, 0x05C, 0x05E, 0x05F, 0x061, 0x063, 0x064, 0x066,
    0x068, 0x06A, 0x06B, 0x06D, 0x06F, 0x071, 0x073, 0x075, 0x076, 0x078, 0x07A, 0x07C, 0x07E, 0x080, 0x082, 0x084,
    0x086, 0x089, 0x08B, 0x08D, 0x08F, 0x091, 0x093, 0x096, 0x098, 0x09A, 0x09C, 0x09F, 0x0A1, 0x0A3, 0x0A6, 0x0A8,
    0x0AB, 0x0AD, 0x0AF, 0x0B2, 0x0B4, 0x0B7, 0x0BA, 0x0BC, 0x0BF, 0x0C1, 0x0C4, 0x0C7, 0x0C9, 0x0CC, 0x0CF, 0x0D2,
    0x0D4, 0x0D7, 0x0DA, 0x0DD, 0x0E0, 0x0E3, 0x0E6, 0x0E9, 0x0EC, 0x0EF, 0x0F2, 0x0F5, 0x0F8, 0x0FB, 0x0FE, 0x101,
    0x104, 0x107, 0x10B, 0x10E, 0x111, 0x114, 0x118, 0x11B, 0x11E, 0x122, 0x125, 0x129, 0x12C, 0x130, 0x133, 0x137,
    0x13A, 0x13E, 0x141, 0x145, 0x148, 0x14C, 0x150, 0x153, 0x157, 0x15B, 0x15F, 0x162, 0x166, 0x16A, 0x16E, 0x172,
    0x176, 0x17A, 0x17D, 0x181, 0x185, 0x189, 0x18D, 0x191, 0x195, 0x19A, 0x19E, 0x1A2, 0x1A6, 0x1AA, 0x1AE, 0x1B2,
    0x1B7, 0x1BB, 0x1BF, 0x1C3, 0x1C8, 0x1CC, 0x1D0, 0x1D5, 0x1D9, 0x1DD, 0x1E2, 0x1E6, 0x1EB, 0x1EF, 0x1F3, 0x1F8,
    0x1FC, 0x201, 0x205, 0x20A, 0x20F, 0x213, 0x218, 0x21C, 0x221, 0x226, 0x22A, 0x22F, 0x233, 0x238, 0x23D, 0x241,
    0x246, 0x24B, 0x250, 0x254, 0x259, 0x25E, 0x263, 0x267, 0x26C, 0x271, 0x276, 0x27B, 0x280, 0x284, 0x289, 0x28E,
    0x293, 0x298, 0x29D, 0x2A2, 0x2A6, 0x2AB, 0x2B0, 0x2B5, 0x2BA, 0x2BF, 0x2C4, 0x2C9, 0x2CE, 0x2D3, 0x2D8, 0x2DC,
    0x2E1, 0x2E6, 0x2EB, 0x2F0, 0x2F5, 0x2FA, 0x2FF, 0x304, 0x309, 0x30E, 0x313, 0x318, 0x31D, 0x322, 0x326, 0x32B,
    0x330, 0x335, 0x33A, 0x33F, 0x344, 0x349, 0x34E, 0x353, 0x357, 0x35C, 0x361, 0x366, 0x36B, 0x370, 0x374, 0x379,
    0x37E, 0x383, 0x388, 0x38C, 0x391, 0x396, 0x39B, 0x39F, 0x3A4, 0x3A9, 0x3AD, 0x3B2, 0x3B7, 0x3BB, 0x3C0, 0x3C5,
    0x3C9, 0x3CE, 0x3D2, 0x3D7, 0x3DC, 0x3E0, 0x3E5, 0x3E9, 0x3ED, 0x3F2, 0x3F6, 0x3FB, 0x3FF, 0x403, 0x408, 0x40C,
    0x410, 0x415, 0x419, 0x41D, 0x421, 0x425, 0x42A, 0x42E, 0x432, 0x436, 0x43A, 0x43E, 0x442, 0x446, 0x44A, 0x44E,
    0x452, 0x455, 0x459, 0x45D, 0x461, 0x465, 0x468, 0x46C, 0x470, 0x473, 0x477, 0x47A, 0x47E, 0x481, 0x485, 0x488,
    0x48C, 0x48F, 0x492, 0x496, 0x499, 0x49C, 0x49F, 0x4A2, 0x4A6, 0x4A9, 0x4AC, 0x4AF, 0x4B2, 0x4B5, 0x4B7, 0x4BA,
    0x4BD, 0x4C0, 0x4C3, 0x4C5, 0x4C8, 0x4CB, 0x4CD, 0x4D0, 0x4D2, 0x4D5, 0x4D7, 0x4D9, 0x4DC, 0x4DE, 0x4E0, 0x4E3,
    0x4E5, 0x4E7, 0x4E9, 0x4EB, 0x4ED, 0x4EF, 0x4F1, 0x4F3, 0x4F5, 0x4F6, 0x4F8, 0x4FA, 0x4FB, 0x4FD, 0x4FF, 0x500,
    0x502, 0x503, 0x504, 0x506, 0x507, 0x508, 0x50A, 0x50B, 0x50C, 0x50D, 0x50E, 0x50F, 0x510, 0x511, 0x511, 0x512,
    0x513, 0x514, 0x514, 0x515, 0x516, 0x516, 0x517, 0x517, 0x517, 0x518, 0x518, 0x518, 0x518, 0x518, 0x519, 0x519
];