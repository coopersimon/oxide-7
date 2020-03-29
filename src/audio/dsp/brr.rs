// Decoding the bit rate reduction format.

use bitflags::bitflags;
use itertools::Itertools;

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
}

// Decode BRR samples. Returns a slice of 16-bit PCM,
// and a bool that indicates whether the sample should loop or not.
#[inline]
pub fn decode_samples(start: u16, ram: &RAM) -> (Box<[i16]>, bool) {
    let mut data = Vec::new();
    let mut should_loop = false;
    let mut last1 = 0;
    let mut last2 = 0;

    for sample in &ram.iter(start as usize).chunks(9) {
        let mut sample_iter = sample.into_iter();
        let head = BRRHead::from_bits_truncate(sample_iter.next().unwrap());
        for d in sample_iter {
            let first = hi_nybble!(d);
            let samp = decompress_sample(head, first, last1, last2);
            data.push(samp);
            last2 = last1;
            last1 = samp;

            let second = lo_nybble!(d);
            let samp = decompress_sample(head, second, last1, last2);
            data.push(samp);
            last2 = last1;
            last1 = samp;
        }
        should_loop = head.contains(BRRHead::LOOP);
        if head.contains(BRRHead::END) {
            break;
        }
    }

    (data.into_boxed_slice(), should_loop)
}

// Clamp val between min and max.
macro_rules! clamp {
    ($val:expr, $min:expr, $max:expr) => {
        std::cmp::min($max, std::cmp::max($min, $val))
    };
}

// Sign extend a 4-bit signed value to 8 bits.
macro_rules! sign_extend {
    ($val:expr) => {
        if test_bit!($val, 3, u8) {
            ($val | 0xF0) as i8
        } else {
            $val as i8
        }
    };
}

#[inline]
fn decompress_sample(head: BRRHead, encoded: u8, last1: i16, last2: i16) -> i16 {
    let unpacked = sign_extend!(encoded) as i16;
    let base = unpacked << head.shift();
    let samp = base + head.calc_coef_a(last1) + head.calc_coef_b(last2);
    clamp!(samp, std::i16::MIN, std::i16::MAX)
}