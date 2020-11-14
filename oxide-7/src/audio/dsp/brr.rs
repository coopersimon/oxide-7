// Decoding the bit rate reduction format.

use bitflags::bitflags;

use crate::mem::RAM;

bitflags! {
    #[derive(Default)]
    struct BRRHead: u8 {
        const RANGE     = bits![7, 6, 5, 4];
        const FILTER    = bits![3, 2];
        const LOOP      = bit!(1);
        const END       = bit!(0);
    }
}

impl BRRHead {
    fn calc_coef_a(&self, old_samp: i16) -> i16 {
        match (*self & BRRHead::FILTER).bits() >> 2 {
            0 => 0,
            1 => old_samp - (old_samp >> 4),
            2 => {
                let s32 = old_samp as i32;
                let res = (s32 * 2) - ((s32 * 3) >> 5);
                res as i16
            },
            3 => {
                let s32 = old_samp as i32;
                let res = (s32 * 2) - ((s32 * 13) >> 6);
                res as i16
            },
            _ => unreachable!()
        }
    }

    fn calc_coef_b(&self, old_samp: i16) -> i16 {
        match (*self & BRRHead::FILTER).bits() >> 2 {
            0 | 1 => 0,
            2 => {
                let s32 = old_samp as i32;
                let res = s32 - (s32 >> 4);
                -res as i16
            },
            3 => {
                let s32 = old_samp as i32;
                let res = s32 - ((s32 * 3) >> 4);
                -res as i16
            },
            _ => unreachable!()
        }
    }

    fn shift(&self) -> u8 {
        (*self & BRRHead::RANGE).bits() >> 4
    }

    fn end(&self) -> bool {
        self.contains(BRRHead::END)
    }

    fn do_loop(&self) -> bool {
        self.contains(BRRHead::LOOP)
    }
}

// A block of 16 BRR-decoded samples.
pub struct SampleBlock {
    samples:    [i16; 16],
    prev_0:     i16,
    prev_1:     i16,

    end:        bool,
    do_loop:    bool,
}

impl SampleBlock {
    pub fn new() -> Self {
        Self {
            samples:    [0; 16],
            prev_0:     0,
            prev_1:     0,

            end:        false,
            do_loop:    false,
        }
    }

    // Decode samples.
    pub fn decode_samples(&mut self, ram: &RAM, addr: u16) {
        let head = BRRHead::from_bits_truncate(ram.read(addr.into()));
        let ram_iter = ram.iter((addr + 1).into()).take(8);
        let sample_iter = self.samples.chunks_mut(2);
        for (data, sample) in ram_iter.zip(sample_iter) {
            let first = hi_nybble!(data);
            let s = decompress_sample(head, first, self.prev_0, self.prev_1);
            self.prev_1 = self.prev_0;
            self.prev_0 = s;
            sample[0] = s;

            let second = lo_nybble!(data);
            let s = decompress_sample(head, second, self.prev_0, self.prev_1);
            self.prev_1 = self.prev_0;
            self.prev_0 = s;
            sample[1] = s;
        }
        self.end = head.end();
        self.do_loop = head.do_loop();
    }

    pub fn read(&self, index: usize) -> i16 {
        self.samples[index]
    }

    pub fn end(&self) -> bool {
        self.end
    }

    pub fn do_loop(&self) -> bool {
        self.do_loop && self.end
    }
}

#[inline]
fn decompress_sample(head: BRRHead, encoded: u8, last1: i16, last2: i16) -> i16 {
    let unpacked = sign_extend_4(encoded) as i16;
    let base = (unpacked << head.shift()) >> 1;
    let samp = base + head.calc_coef_a(last1) + head.calc_coef_b(last2);
    sign_extend_15(samp)
}

// Sign extend a 4-bit signed value to 8 bits.
#[inline]
fn sign_extend_4(val: u8) -> i8 {
    if test_bit!(val, 3, u8) {
        (val | 0xF0) as i8
    } else {
        val as i8
    }
}

// Take the lower 15 bits of a value and sign extend to 16 bits.
fn sign_extend_15(val: i16) -> i16 {
    let clipped = clamp!(val, -0x4000, 0x3FFF);
    let top_bit = clipped & (bit!(14, u16) as i16);
    clipped | (top_bit << 1)
}