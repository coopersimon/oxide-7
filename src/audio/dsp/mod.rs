// Digital signal processor

// Clamp val between min and max.
macro_rules! clamp {
    ($val:expr, $min:expr, $max:expr) => {
        std::cmp::min($max, std::cmp::max($min, $val))
    };
}

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
const SAMPLE_CYCLES: usize = super::SPC_CLOCK_RATE / SPC_SAMPLE_RATE;
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

    echo_feedback:  i8,
    pitch_mod:      u8,

    noise_enable:   u8,
    echo_enable:    u8,

    src_offset:     u8,
    echo_offset:    u8,
    echo_delay:     u8,
    echo_internal_counter:  u16,
    echo_fir_coefs: [i8; 8],
}

pub struct DSP {
    signal_tx:      Sender<super::SamplePacket>,
    cycle_count:    usize,
    frames:         Vec<Stereo<f32>>,

    echo_buffer_size:   u16,

    fir_buffer:         [Stereo<i16>; 8],
    fir_buffer_index:   usize,

    noise_level:        i16,
    noise_step:         Option<usize>,
    noise_count:        usize,

    regs:           DSPRegisters,
    voices:         [Voice; 8],
}

impl DSP {
    pub fn new(signal_tx: Sender<super::SamplePacket>) -> Self {
        DSP {
            signal_tx:      signal_tx,
            cycle_count:    0,
            frames:         Vec::with_capacity(SAMPLE_BATCH_SIZE),

            echo_buffer_size:   0,

            fir_buffer:         [Stereo::equilibrium(); 8],
            fir_buffer_index:   0,

            noise_level:        -0x4000,
            noise_step:         None,
            noise_count:        0,

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

    pub fn clock(&mut self, cycles: usize, ram: &mut RAM) {
        // Generate a new sample every 32 cycles.
        self.cycle_count += cycles;
        if self.cycle_count >= SAMPLE_CYCLES {
            self.generate_frame(ram);
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

            0x0D => self.regs.echo_feedback as u8,
            0x2D => self.regs.pitch_mod,
            0x3D => self.regs.noise_enable,
            0x4D => self.regs.echo_enable,
            0x5D => self.regs.src_offset,
            0x6D => self.regs.echo_offset,
            0x7D => self.regs.echo_delay,

            _ if lo_nybble!(addr) == 0xF => self.regs.echo_fir_coefs[(hi_nybble!(addr) as usize) & 0x7] as u8,

            0x00..=0x7F => self.voices[(addr >> 4) as usize].read(addr),

            _ => 0
        }
    }

    pub fn write(&mut self, addr: u8, data: u8, ram: &RAM) {
        match addr {
            0x0C => self.set_main_vol_left(data),
            0x1C => self.set_main_vol_right(data),
            0x2C => self.set_echo_vol_left(data),
            0x3C => self.set_echo_vol_right(data),
            0x4C => self.set_key_on(data, ram),
            0x5C => self.set_key_off(data),
            0x6C => self.set_flags(data),
            0x7C => self.regs.endx = data,

            0x0D => self.regs.echo_feedback = data as i8,
            0x2D => self.set_pitch_mod(data),
            0x3D => self.set_noise_enable(data),
            0x4D => self.set_echo_enable(data),
            0x5D => self.regs.src_offset = data,
            0x6D => self.regs.echo_offset = data,
            0x7D => self.set_echo_delay(data),

            _ if lo_nybble!(addr) == 0xF => self.regs.echo_fir_coefs[(hi_nybble!(addr) as usize) & 0x7] = data as i8,

            0x00..=0x7F => self.voices[(addr >> 4) as usize].write(addr, data),

            _ => {}
        }
    }
}

impl DSP {
    // Generate a single left-right pair of audio samples.
    fn generate_frame(&mut self, ram: &mut RAM) {
        const MIN: i32 = std::i16::MIN as i32;
        const MAX: i32 = std::i16::MAX as i32;

        let mut main_left = 0;  // Main signal
        let mut main_right = 0;
        let mut prev = 0;       // Previous channel's sample for pitch modulation
        let mut echo_left = 0;  // Echo signal
        let mut echo_right = 0;

        for voice in &mut self.voices {
            if let Some(v) = voice.generate(prev, self.noise_level) {
                prev = v;

                let v_samp = v as i32;
                let voice_left = clamp!((v_samp * voice.read_left_vol()) >> 6, MIN, MAX);
                let voice_right = clamp!((v_samp * voice.read_right_vol()) >> 6, MIN, MAX);
                main_left += voice_left;
                main_right += voice_right;

                if voice.is_echo_enabled() {
                    echo_left += voice_left;
                    echo_right += voice_right;
                }
            } else {
                prev = 0;
            }
        }

        self.step_noise();

        let echo = self.generate_echo(ram, echo_left as i16, echo_right as i16);

        let frame = if self.is_mute() {
            Stereo::equilibrium()
        } else {
            let mut left = clamp!((main_left * (self.regs.main_vol_left as i32)) >> 7, MIN, MAX);
            let mut right = clamp!((main_right * (self.regs.main_vol_right as i32)) >> 7, MIN, MAX);
            left += clamp!(((echo[0] as i32) * (self.regs.echo_vol_left as i32)) >> 7, MIN, MAX);
            right += clamp!(((echo[1] as i32) * (self.regs.echo_vol_right as i32)) >> 7, MIN, MAX);
            left = clamp!(left, MIN, MAX);
            right = clamp!(right, MIN, MAX);
            [(left as f32) / 32_768.0, (right as f32) / 32_768.0]
        };

        self.frames.push(frame);
    }

    // Generate a single echo frame based on the main output.
    fn generate_echo(&mut self, ram: &mut RAM, main_left: i16, main_right: i16) -> Stereo<i16> {
        let echo_buffer_address = make16!(self.regs.echo_offset, 0).wrapping_add(self.regs.echo_internal_counter);
        let buffer_samples = (0..4).map(|i| ram.read(echo_buffer_address.wrapping_add(i) as u32)).collect::<Box<[_]>>();
        let buffer_sample_left = make16!(buffer_samples[1], buffer_samples[0]) as i16;
        let buffer_sample_right = make16!(buffer_samples[3], buffer_samples[2]) as i16;

        let echo_val = self.calculate_fir([buffer_sample_left, buffer_sample_right]);

        // Write new samples.
        if !self.regs.flags.contains(DSPFlags::ECHO_WRITES) {
            let feedback_vol = self.regs.echo_feedback as i32;
            let feedback_left = (((echo_val[0] as i32) * feedback_vol) >> 7) as i16;
            let feedback_right = (((echo_val[1] as i32) * feedback_vol) >> 7) as i16;

            let buffer_input_left = main_left + feedback_left;
            let buffer_input_right = main_right + feedback_right;

            ram.write(echo_buffer_address as u32, lo!(buffer_input_left));
            ram.write(echo_buffer_address.wrapping_add(1) as u32, hi!(buffer_input_left));
            ram.write(echo_buffer_address.wrapping_add(2) as u32, lo!(buffer_input_right));
            ram.write(echo_buffer_address.wrapping_add(3) as u32, hi!(buffer_input_right));
        }

        self.regs.echo_internal_counter = self.regs.echo_internal_counter.wrapping_add(4);
        if self.regs.echo_internal_counter >= self.echo_buffer_size {
            self.regs.echo_internal_counter = 0;
        }

        echo_val
    }

    fn calculate_fir(&mut self, new_sample: Stereo<i16>) -> Stereo<i16> {
        self.fir_buffer[self.fir_buffer_index] = new_sample;

        let out = (0..8).fold(Stereo::equilibrium(), |acc: Stereo<i32>, i| {
            let index = self.fir_buffer_index.wrapping_sub(7 - i) & 7;
            let buffer_sample = self.fir_buffer[index];
            let fir_coef = self.regs.echo_fir_coefs[i] as i32;
            let left = ((buffer_sample[0] as i32) * fir_coef) >> 7;
            let right = ((buffer_sample[1] as i32) * fir_coef) >> 7;
            [acc[0] + left, acc[1] + right]
        });

        //let left = out[0] + (new_sample[0] as i32) * (self.regs.echo_fir_coefs[7] as i32);
        //let right = out[1] + (new_sample[1] as i32) * (self.regs.echo_fir_coefs[7] as i32);

        let left_clamped = clamp!(out[0], std::i16::MIN as i32, std::i16::MAX as i32) as i16;
        let right_clamped = clamp!(out[1], std::i16::MIN as i32, std::i16::MAX as i32) as i16;

        self.fir_buffer_index = (self.fir_buffer_index + 1) & 7;

        [left_clamped, right_clamped]
    }

    fn step_noise(&mut self) {
        if let Some(step) = self.noise_step {
            self.noise_count += 1;
            if self.noise_count >= step {
                self.noise_count = 0;

                let new_top_bit = (self.noise_level ^ (self.noise_level >> 1)) & 1;
                let new_level = ((self.noise_level >> 1) & 0x3FFF) | (new_top_bit << 14) | (new_top_bit << 15);
                self.noise_level = new_level;
            }
        }
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
        self.noise_step = envelope::step_size((self.regs.flags & DSPFlags::NOISE_FREQ).bits());
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
    }

    fn set_main_vol_right(&mut self, val: u8) {
        self.regs.main_vol_right = val;
    }

    fn set_echo_vol_left(&mut self, val: u8) {
        self.regs.echo_vol_left = val;
    }

    fn set_echo_vol_right(&mut self, val: u8) {
        self.regs.echo_vol_right = val;
    }

    fn set_echo_enable(&mut self, val: u8) {
        self.regs.echo_enable = val;
        for i in 0..8 {
            self.voices[i].enable_echo(test_bit!(val, i, u8));
        }
    }

    fn set_echo_delay(&mut self, val: u8) {
        const ECHO_BUFFER_STEP_SIZE: u16 = 2048;

        self.regs.echo_delay = val;
        self.echo_buffer_size = if (val & 0xF) == 0 {
            4
        } else {
            ((val & 0xF) as u16) * ECHO_BUFFER_STEP_SIZE
        };
    }
}
