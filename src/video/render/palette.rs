// Palette caches for decoded CGRAM.
use crate::video::{
    VideoMem,
    render::Colour
};

const MAX_COLOUR: u16 = 0x1F;
macro_rules! col15_to_col888 {
    ($rgb:expr) => {
        {
            let r = ($rgb & MAX_COLOUR) << 3;
            let g = (($rgb >> 5) & MAX_COLOUR) << 3;
            let b = (($rgb >> 10) & MAX_COLOUR) << 3;
            Colour::new(r as u8, g as u8, b as u8)
        }
    };
}

pub struct PaletteMem {
    bg_colours: [Colour; 128],
    obj_colours: [Colour; 128]
}

impl PaletteMem {
    pub fn new() -> Self {
        PaletteMem {
            bg_colours: [Colour::zero(); 128],
            obj_colours: [Colour::zero(); 128]
        }
    }

    pub fn make_bg_palette(&mut self, mem: &VideoMem) {
        for (d, p) in mem.get_cgram().chunks(2).take(128).zip(self.bg_colours.iter_mut()) {
            *p = col15_to_col888!(make16!(d[1], d[0])); // TODO: avoid allocing new colour (?)
        }
    }

    pub fn make_obj_palette(&mut self, mem: &VideoMem) {
        for (d, p) in mem.get_cgram().chunks(2).skip(128).zip(self.obj_colours.iter_mut()) {
            *p = col15_to_col888!(make16!(d[1], d[0])); // TODO: avoid allocing new colour (?)
        }
    }

    pub fn get_bg_colour(&self, which: usize) -> Colour {
        self.bg_colours[which]
    }

    pub fn get_obj_colour(&self, which: usize) -> Colour {
        self.obj_colours[which]
    }

    // TODO: more indexing..?
}