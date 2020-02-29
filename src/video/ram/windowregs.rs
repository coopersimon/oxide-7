// Video registers for colour math and window settings.
use crate::video::render::Colour;

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct WindowMaskSettings: u8 {
        const WINDOW_2_ENABLE_HI = bit!(7);
        const WINDOW_2_INVERT_HI = bit!(6);
        const WINDOW_1_ENABLE_HI = bit!(5);
        const WINDOW_1_INVERT_HI = bit!(4);
        const WINDOW_2_ENABLE_LO = bit!(3);
        const WINDOW_2_INVERT_LO = bit!(2);
        const WINDOW_1_ENABLE_LO = bit!(1);
        const WINDOW_1_INVERT_LO = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    pub struct LayerDesignation: u8 {
        const OBJ = bit!(4);
        const BG4 = bit!(3);
        const BG3 = bit!(2);
        const BG2 = bit!(1);
        const BG1 = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    pub struct ColourMathDesignation: u8 {
        const ADD_SUB   = bit!(7);
        const HALF      = bit!(6);
        const BACKDROP  = bit!(5);
        const OBJ       = bit!(4);
        const BG4       = bit!(3);
        const BG3       = bit!(2);
        const BG2       = bit!(1);
        const BG1       = bit!(0);
    }
}

const FIXED_COLOUR_B_BIT: u8 = 7;
const FIXED_COLOUR_G_BIT: u8 = 6;
const FIXED_COLOUR_R_BIT: u8 = 5;
const MAX_COLOUR: u8 = 0x1F;

pub struct WindowRegisters {
    mask_bg1_2:         WindowMaskSettings,
    mask_bg3_4:         WindowMaskSettings,
    mask_obj_col:       WindowMaskSettings,

    pub window_1_left:  u8,
    pub window_1_right: u8,
    pub window_2_left:  u8,
    pub window_2_right: u8,

    mask_logic_bg:      u8,
    mask_logic_obj_col: u8,
    
    main_screen_desg:   LayerDesignation,
    sub_screen_desg:    LayerDesignation,

    main_mask_desg:     LayerDesignation,
    sub_mask_desg:      LayerDesignation,

    colour_add_select:  u8,
    colour_math_desg:   ColourMathDesignation,

    fixed_colour:       Colour,
}

impl WindowRegisters {
    pub fn new() -> Self {
        WindowRegisters {
            mask_bg1_2:         WindowMaskSettings::default(),
            mask_bg3_4:         WindowMaskSettings::default(),
            mask_obj_col:       WindowMaskSettings::default(),

            window_1_left:      0,
            window_1_right:     0,
            window_2_left:      0,
            window_2_right:     0,

            mask_logic_bg:      0,
            mask_logic_obj_col: 0,
            
            main_screen_desg:   LayerDesignation::default(),
            sub_screen_desg:    LayerDesignation::default(),

            main_mask_desg:     LayerDesignation::default(),
            sub_mask_desg:      LayerDesignation::default(),

            colour_add_select:  0,
            colour_math_desg:   ColourMathDesignation::default(),

            fixed_colour:       Colour::zero(),
        }
    }

    // Setters - CPU side
    pub fn set_mask_bg1_2(&mut self, data: u8) {
        self.mask_bg1_2 = WindowMaskSettings::from_bits_truncate(data);
    }

    pub fn set_mask_bg3_4(&mut self, data: u8) {
        self.mask_bg3_4 = WindowMaskSettings::from_bits_truncate(data);
    }

    pub fn set_mask_obj_col(&mut self, data: u8) {
        self.mask_obj_col = WindowMaskSettings::from_bits_truncate(data);
    }

    pub fn set_mask_logic_bg(&mut self, data: u8) {
        self.mask_logic_bg = data;
    }

    pub fn set_mask_logic_obj_col(&mut self, data: u8) {
        self.mask_logic_obj_col = data;
    }

    pub fn set_main_screen_desg(&mut self, data: u8) {
        self.main_screen_desg = LayerDesignation::from_bits_truncate(data);
    }

    pub fn set_sub_screen_desg(&mut self, data: u8) {
        self.sub_screen_desg = LayerDesignation::from_bits_truncate(data);
    }

    pub fn set_main_mask_desg(&mut self, data: u8) {
        self.main_mask_desg = LayerDesignation::from_bits_truncate(data);
    }

    pub fn set_sub_mask_desg(&mut self, data: u8) {
        self.sub_mask_desg = LayerDesignation::from_bits_truncate(data);
    }

    pub fn set_colour_add_select(&mut self, data: u8) {
        self.colour_add_select = data;
    }

    pub fn set_colour_math_desg(&mut self, data: u8) {
        self.colour_math_desg = ColourMathDesignation::from_bits_truncate(data);
    }

    pub fn set_fixed_colour(&mut self, data: u8) {
        if test_bit!(data, FIXED_COLOUR_B_BIT, u8) {
            let b = data & MAX_COLOUR;
            self.fixed_colour.b = (b << 3) + (b >> 2);
        }
        if test_bit!(data, FIXED_COLOUR_G_BIT, u8) {
            let g = data & MAX_COLOUR;
            self.fixed_colour.g = (g << 3) + (g >> 2);
        }
        if test_bit!(data, FIXED_COLOUR_R_BIT, u8) {
            let r = data & MAX_COLOUR;
            self.fixed_colour.r = (r << 3) + (r >> 2);
        }
    }

    // Getters - renderer side
    pub fn get_fixed_colour(&self) -> Colour {
        self.fixed_colour
    }
}