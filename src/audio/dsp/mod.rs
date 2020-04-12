// Digital signal processor

mod brr;
mod envelope;
mod voice;

use bitflags::bitflags;
use crossbeam_channel::Sender;
use sample::frame::{
    Frame,
    Stereo
};

pub use voice::*;

use crate::mem::RAM;

bitflags! {
    pub struct DSPFlags: u8 {
        const SOFT_RESET    = bit!(7);
        const MUTE          = bit!(6);
        const ECHO_WRITES   = bit!(5);
        const NOISE_FREQ    = bits![4, 3, 2, 1, 0];
    }
}

impl Default for DSPFlags {
    fn default() -> Self {
        DSPFlags::SOFT_RESET | DSPFlags::MUTE | DSPFlags::ECHO_WRITES
    }
}

//const VIDEO_FRAME_CYCLES: f32 = (super::spcthread::SPC_CLOCK_RATE as f32) / 60.0;
const SPC_SAMPLE_RATE: usize = 32_000;
const SAMPLE_CYCLES: usize = super::spcthread::SPC_CLOCK_RATE / SPC_SAMPLE_RATE;
const SAMPLE_BATCH_SIZE: usize = 512;

#[derive(Default, Clone)]
pub struct DSPRegisters {

    main_vol_left:  u8,
    main_vol_right: u8,
    echo_vol_left:  u8,
    echo_vol_right: u8,

    flags:          DSPFlags,

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

pub struct DSP {
    signal_tx:      Sender<super::SamplePacket>,
    cycle_count:    usize,
    frames:         Vec<Stereo<f32>>,

    vol_left:       f32,
    vol_right:      f32,

    regs:           DSPRegisters,
    voices:         [Voice; 8],
}

impl DSP {
    pub fn new(signal_tx: Sender<super::SamplePacket>) -> Self {
        DSP {
            signal_tx:      signal_tx,
            cycle_count:    0,
            frames:         Vec::with_capacity(SAMPLE_BATCH_SIZE),

            vol_left:       0.0,
            vol_right:      0.0,

            regs:           DSPRegisters::default(),
            voices:         [
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
            ],
        }
    }

    pub fn clock(&mut self, cycles: usize) {
        // Generate a new sample every 32 cycles.
        self.cycle_count += cycles;
        if self.cycle_count >= SAMPLE_CYCLES {
            self.generate_frame();
            self.cycle_count -= SAMPLE_CYCLES;
        }

        // Every 16384 cycles, send the batch of 512 samples over to the audio thread.
        if self.frames.len() >= SAMPLE_BATCH_SIZE {
            let in_ = self.frames.drain(..).collect::<Box<[_]>>();
            self.signal_tx.send(in_).unwrap();
        }
    }

    pub fn read(&self, addr: u8) -> u8 {
        //println!("Reading from DSP {:X}", addr);
        match addr {
            0x0C => self.regs.main_vol_left,
            0x1C => self.regs.main_vol_right,
            0x2C => self.regs.echo_vol_left,
            0x3C => self.regs.echo_vol_right,
            0x4C => self.regs.key_on,
            0x5C => self.regs.key_off,
            0x6C => self.regs.flags.bits(),
            0x7C => self.read_endx(),

            0x0D => self.regs.echo_feedback,
            0x2D => self.regs.pitch_mod,
            0x3D => self.regs.noise_enable,
            0x4D => self.regs.echo_enable,
            0x5D => self.regs.src_offset,
            0x6D => self.regs.echo_offset,
            0x7D => self.regs.echo_delay,

            0x00..=0x7F => self.voices[(addr >> 4) as usize].read(addr),

            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u8, data: u8, ram: &RAM) {
        match addr {
            0x0C => self.set_main_vol_left(data),
            0x1C => self.set_main_vol_right(data),
            0x2C => self.regs.echo_vol_left = data,
            0x3C => self.regs.echo_vol_right = data,
            0x4C => self.set_key_on(data, ram),
            0x5C => self.set_key_off(data),
            0x6C => self.set_flags(data),
            0x7C => self.regs.endx = data,

            0x0D => self.regs.echo_feedback = data,
            0x2D => self.set_pitch_mod(data),
            0x3D => self.set_noise_enable(data),
            0x4D => self.set_echo_enable(data),
            0x5D => self.regs.src_offset = data,
            0x6D => self.regs.echo_offset = data,
            0x7D => self.regs.echo_delay = data,

            0x00..=0x7F => self.voices[(addr >> 4) as usize].write(addr, data),

            _ => {}
        }
    }
}

impl DSP {
    fn generate_frame(&mut self) {
        let mut left = 0.0;
        let mut right = 0.0;
        let mut prev = 0;

        for voice in &mut self.voices {
            if let Some(v) = voice.generate(prev) {
                prev = v;

                let v_samp = (v as f32) / 32_768.0;
                let voice_left = v_samp * voice.read_left_vol();
                let voice_right = v_samp * voice.read_right_vol();
                left += voice_left / 8.0;
                right += voice_right / 8.0;
            } else {
                prev = 0;
            }
        }

        let frame = if self.is_mute() {
            Stereo::equilibrium()
        } else {
            [left * self.vol_left, right * self.vol_right]
        };

        self.frames.push(frame);
    }
}

impl DSP {
    fn set_key_on(&mut self, val: u8, ram: &RAM) {
        self.regs.key_on = val;
        for v in 0..8 {
            if test_bit!(val, v, u8) {
                let (sample, should_loop) = brr::decode_samples(self.get_sample_addr(v, ram), ram);
                let s_loop = if should_loop {
                    let (s_loop, _) = brr::decode_samples(self.get_loop_addr(v, ram), ram);
                    s_loop
                } else { Box::new([]) };

                self.voices[v].key_on(sample, s_loop);
            }
        }
    }

    fn set_key_off(&mut self, val: u8) {
        self.regs.key_off = val;
        for v in 0..8 {
            if test_bit!(val, v, u8) {
                self.voices[v].key_off();
            }
        }
    }

    fn get_sample_addr(&self, voice_num: usize, ram: &RAM) -> u16 {
        let dir_index = self.voices[voice_num].dir_index();
        let source_dir_addr = make16!(self.regs.src_offset, 0).wrapping_add(dir_index) as u32;
        let addr_lo = ram.read(source_dir_addr);
        let addr_hi = ram.read(source_dir_addr + 1);
        make16!(addr_hi, addr_lo)
    }

    fn get_loop_addr(&self, voice_num: usize, ram: &RAM) -> u16 {
        let dir_index = self.voices[voice_num].dir_index();
        let source_dir_addr = make16!(self.regs.src_offset, 0).wrapping_add(dir_index) as u32;
        let addr_lo = ram.read(source_dir_addr + 2);
        let addr_hi = ram.read(source_dir_addr + 3);
        make16!(addr_hi, addr_lo)
    }

    fn read_endx(&self) -> u8 {
        (0..8).fold(0, |acc, v| {
            let end = if self.voices[v].is_on() { 0 } else { bit!(v) };
            acc | end
        })
    }

    fn set_flags(&mut self, val: u8) {
        self.regs.flags = DSPFlags::from_bits_truncate(val);
        if self.regs.flags.contains(DSPFlags::SOFT_RESET) {
            self.set_key_off(0xFF);
        }
    }

    fn is_mute(&self) -> bool {
        self.regs.flags.contains(DSPFlags::MUTE)
    }

    fn set_noise_enable(&mut self, val: u8) {
        self.regs.noise_enable = val;
        for i in 0..8 {
            self.voices[i].enable_noise(test_bit!(val, i, u8));
        }
    }

    fn set_pitch_mod(&mut self, val: u8) {
        self.regs.pitch_mod = val;
        for i in 1..8 {
            self.voices[i].enable_pitch_mod(test_bit!(val, i, u8));
        }
    }

    fn set_main_vol_left(&mut self, val: u8) {
        self.regs.main_vol_left = val;
        self.vol_left = ((val as i8) as f32) / 128.0;
    }

    fn set_main_vol_right(&mut self, val: u8) {
        self.regs.main_vol_right = val;
        self.vol_right = ((val as i8) as f32) / 128.0;
    }

    fn set_echo_enable(&mut self, val: u8) {
        self.regs.echo_enable = val;
    }
}
