// Palette caches for decoded CGRAM.
use crate::video::{
    VideoMem,
    render::Colour
};

const MAX_COLOUR: u16 = 0x1F;
macro_rules! col15_to_col888 {
    ($rgb:expr) => {
        {
            let r_i = ($rgb & MAX_COLOUR) << 3;
            let g_i = (($rgb >> 5) & MAX_COLOUR) << 3;
            let b_i = (($rgb >> 10) & MAX_COLOUR) << 3;
            let r = r_i | (r_i >> 5);
            let g = g_i | (g_i >> 5);
            let b = b_i | (b_i >> 5);
            Colour::new(r as u8, g as u8, b as u8)
        }
    };
}

pub struct PaletteMem {
    colours: [Colour; 256]
}

impl PaletteMem {
    pub fn new() -> Self {
        PaletteMem {
            colours: [Colour::zero(); 256],
        }
    }

    pub fn make_bg_palette(&mut self, mem: &VideoMem) {
        for (d, p) in mem.get_cgram().chunks(2).take(128).zip(self.colours.iter_mut()) {
            *p = col15_to_col888!(make16!(d[1], d[0]));
        }
    }

    pub fn make_obj_palette(&mut self, mem: &VideoMem) {
        for (d, p) in mem.get_cgram().chunks(2).skip(128).zip(self.colours.iter_mut().skip(128)) {
            *p = col15_to_col888!(make16!(d[1], d[0]));
        }
    }

    pub fn get_bg_colour(&self, which: usize) -> Colour {
        self.colours[which]
    }

    pub fn get_obj_colour(&self, which: usize) -> Colour {
        self.colours[which + 128]
    }
}