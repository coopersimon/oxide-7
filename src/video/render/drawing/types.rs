// Types to assist with drawing.

use crate::video::render::Colour;

// A sprite colour for final compositing.
#[derive(Clone, Copy)]
pub struct SpriteColour {
    pub colour:     Colour, // The colour of the sprite
    pub col_math:   bool    // Should it participate in colour math
}

// A sprite pixel.
#[derive(Clone, Copy)]
pub enum SpritePixel {
    // Colours with respective priorities.
    Prio3(SpriteColour),
    Prio2(SpriteColour),
    Prio1(SpriteColour),
    Prio0(SpriteColour),
    Masked,                 // This pixel is masked and no sprites should appear here.
    None                    // No sprites found on this pixel.
}

impl SpritePixel {
    // Test if the pixel is masked.
    #[inline]
    pub fn is_masked(&self) -> bool {
        match self {
            SpritePixel::Masked => true,
            _ => false
        }
    }

    // Get the final pixel out, and test it to see if it should participate in colour math.
    pub fn pixel(&self) -> Pixel {
        match self {
            SpritePixel::Prio3(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            SpritePixel::Prio2(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            SpritePixel::Prio1(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            SpritePixel::Prio0(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            _ => panic!("Don't call this on pixels with no colour value.")
        }
    }
}

// A colour value with source information.
pub enum Pixel {
    BG1(Colour),
    BG2(Colour),
    BG3(Colour),
    BG4(Colour),
    ObjHi(Colour),  // Uses object palette 4-7, uses colour math
    ObjLo(Colour),  // Uses object palette 0-3, doesn't use colmath
    None
}

impl Pixel {
    // Get the colour from if there is one, and discard source information.
    pub fn any(self) -> Option<Colour> {
        match self {
            Pixel::BG1(c) => Some(c),
            Pixel::BG2(c) => Some(c),
            Pixel::BG3(c) => Some(c),
            Pixel::BG4(c) => Some(c),
            Pixel::ObjHi(c) => Some(c),
            Pixel::ObjLo(c) => Some(c),
            Pixel::None => None
        }
    }
}
