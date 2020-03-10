// Digital signal processor

mod consts;
mod voice;
use voice::Voice;

pub struct DSP {
    voices: [Voice; 8],

    main_vol_left:  u8,
    main_vol_right: u8,
    echo_vol_left:  u8,
    echo_vol_right: u8,

    flags:          u8,

    key_on:         u8,
    key_off:        u8,
    endx:           u8,

    echo_feedback:  u8,
    pitch_mod:      u8,

    noise_enable:   u8,
    echo_enable:    u8,

    src_offset:     u8,
    echo_offset:    u8,
    echo_delay:     u8,
}

impl DSP {
    pub fn new() -> Self {
        DSP {
            voices: [Voice::new(); 8],

            main_vol_left:  0,
            main_vol_right: 0,
            echo_vol_left:  0,
            echo_vol_right: 0,

            flags:          0,

            key_on:         0,
            key_off:        0,
            endx:           0,

            echo_feedback:  0,
            pitch_mod:      0,

            noise_enable:   0,
            echo_enable:    0,

            src_offset:     0,
            echo_offset:    0,
            echo_delay:     0,
        }
    }

    pub fn read(&self, addr: u8) -> u8 {
        match addr {
            0x0C => self.main_vol_left,
            0x1C => self.main_vol_right,
            0x2C => self.echo_vol_left,
            0x3C => self.echo_vol_right,
            0x4C => self.key_on,
            0x5C => self.key_off,
            0x6C => self.flags,
            0x7C => self.endx,

            0x0D => self.echo_feedback,
            0x2D => self.pitch_mod,
            0x3D => self.noise_enable,
            0x4D => self.echo_enable,
            0x5D => self.src_offset,
            0x6D => self.echo_offset,
            0x7D => self.echo_delay,

            0x00..=0x7F => self.voices[(addr >> 4) as usize].read(addr),

            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x0C => self.main_vol_left = data,
            0x1C => self.main_vol_right = data,
            0x2C => self.echo_vol_left = data,
            0x3C => self.echo_vol_right = data,
            0x4C => self.key_on = data,
            0x5C => self.key_off = data,
            0x6C => self.flags = data,
            0x7C => self.endx = data,

            0x0D => self.echo_feedback = data,
            0x2D => self.pitch_mod = data,
            0x3D => self.noise_enable = data,
            0x4D => self.echo_enable = data,
            0x5D => self.src_offset = data,
            0x6D => self.echo_offset = data,
            0x7D => self.echo_delay = data,

            0x00..=0x7F => self.voices[(addr >> 4) as usize].write(addr, data),

            _ => {}
        }
    }
}