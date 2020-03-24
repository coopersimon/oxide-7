// Digital signal processor

mod brr;
mod voice;

use bitflags::bitflags;
use crossbeam_channel::Sender;

pub use voice::*;

use super::generator::{
    AudioData, VoiceData
};

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

const VIDEO_FRAME_CYCLES: f32 = (super::spcthread::SPC_CLOCK_RATE as f32) / 60.0;

#[derive(Default, Clone)]
pub struct DSPRegisters {

    main_vol_left:  u8,
    main_vol_right: u8,
    echo_vol_left:  u8,
    echo_vol_right: u8,

    flags:          DSPFlags,

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
    signal_tx:      Sender<AudioData>,
    cycle_count:    f32,

    voices:         [Voice; 8],

    regs:           DSPRegisters,
}

impl DSP {
    pub fn new(signal_tx: Sender<AudioData>) -> Self {
        DSP {
            signal_tx:      signal_tx,
            cycle_count:    0.0,

            voices:         [Voice::new(); 8],

            regs:           DSPRegisters::default()
        }
    }

    pub fn clock(&mut self, cycles: usize) {
        self.cycle_count += cycles as f32;
        if self.cycle_count >= VIDEO_FRAME_CYCLES {
            self.cycle_count -= VIDEO_FRAME_CYCLES;
            self.signal_tx.send(AudioData::Frame).expect("Couldn't send frame signal to audio generator");
        }
    }

    pub fn read(&self, addr: u8) -> u8 {
        //println!("Reading from DSP {:X}", addr);
        match addr {
            0x0C => self.regs.main_vol_left,
            0x1C => self.regs.main_vol_right,
            0x2C => self.regs.echo_vol_left,
            0x3C => self.regs.echo_vol_right,
            //0x4C => self.regs.key_on,
            //0x5C => self.regs.key_off,
            0x6C => self.regs.flags.bits(),
            0x7C => self.regs.endx,

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
    fn set_key_on(&mut self, val: u8, ram: &RAM) {
        for v in 0..8 {
            if test_bit!(val, v, u8) {
                let (sample, should_loop) = brr::decode_samples(self.get_sample_addr(v, ram), ram);
                let s_loop = if should_loop {
                    let (s_loop, _) = brr::decode_samples(self.get_loop_addr(v, ram), ram);
                    s_loop
                } else { Box::new([]) };

                self.signal_tx.send(AudioData::VoiceKeyOn{
                    data: VoiceData {
                        regs:   Box::new(self.voices[v]),
                        sample: sample,
                        s_loop: s_loop,
                    },
                    num:  v,
                    time: self.cycle_count / VIDEO_FRAME_CYCLES
                }).expect("Couldn't send key on signal to audio generator");
            }
        }
    }

    fn set_key_off(&mut self, val: u8) {
        for v in 0..8 {
            if test_bit!(val, v, u8) {
                self.signal_tx.send(AudioData::VoiceKeyOff{
                    num:  v,
                    time: self.cycle_count / VIDEO_FRAME_CYCLES
                }).expect("Couldn't send key on signal to audio generator");
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

    fn set_flags(&mut self, val: u8) {
        let new_flags = DSPFlags::from_bits_truncate(val);
        if self.regs.flags.contains(DSPFlags::SOFT_RESET) {
            self.set_key_off(0xFF);
        }
        if new_flags.contains(DSPFlags::MUTE) && !self.regs.flags.contains(DSPFlags::MUTE) {
            self.signal_tx.send(AudioData::Mute(true)).expect("Couldn't mute");
        } else if !new_flags.contains(DSPFlags::MUTE) && self.regs.flags.contains(DSPFlags::MUTE) {
            self.signal_tx.send(AudioData::Mute(false)).expect("Couldn't unmute");
        }

        self.regs.flags = new_flags;
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
        let vol = ((val as i8) as f32) / 128.0;
        self.signal_tx.send(AudioData::DSPVolLeft(vol)).expect("Couldn't send vol left");
    }

    fn set_main_vol_right(&mut self, val: u8) {
        self.regs.main_vol_right = val;
        let vol = ((val as i8) as f32) / 128.0;
        self.signal_tx.send(AudioData::DSPVolRight(vol)).expect("Couldn't send vol right");
    }

    fn set_echo_enable(&mut self, val: u8) {
        self.regs.echo_enable = val;
        //println!("Echo: {:X}", val);
    }
}
