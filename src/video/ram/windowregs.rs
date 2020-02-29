// Video registers for colour math and window settings.
use crate::video::render::Colour;

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct WindowMaskSettings: u8 {
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
    struct BGMaskLogic: u8 {
        const BG_4_OP = bits![7, 6];
        const BG_3_OP = bits![5, 4];
        const BG_2_OP = bits![3, 2];
        const BG_1_OP = bits![1, 0];
    }
}

bitflags! {
    #[derive(Default)]
    struct ObjColMaskLogic: u8 {
        const COL_OP = bits![3, 2];
        const OBJ_OP = bits![1, 0];
    }
}

const MASK_LOGIC_OR: u8 = 0;
const MASK_LOGIC_AND: u8 = 1;
const MASK_LOGIC_XOR: u8 = 2;
const MASK_LOGIC_XNOR: u8 = 3;

bitflags! {
    #[derive(Default)]
    struct LayerDesignation: u8 {
        const OBJ = bit!(4);
        const BG4 = bit!(3);
        const BG3 = bit!(2);
        const BG2 = bit!(1);
        const BG1 = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    struct ColourMathDesignation: u8 {
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

    mask_logic_bg:      BGMaskLogic,
    mask_logic_obj_col: ObjColMaskLogic,
    
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

            mask_logic_bg:      BGMaskLogic::default(),
            mask_logic_obj_col: ObjColMaskLogic::default(),
            
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
        self.mask_logic_bg = BGMaskLogic::from_bits_truncate(data);
    }

    pub fn set_mask_logic_obj_col(&mut self, data: u8) {
        self.mask_logic_obj_col = ObjColMaskLogic::from_bits_truncate(data);
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

    // Returns true if the pixel should be shown for the bg specified on the main screen.
    pub fn show_bg_pixel_main(&self, bg: usize, x: u8) -> bool {
        if self.enable_bg_main(bg) {    // Check if this bg is enabled for the main screen.
            if self.enable_masking_bg_main(bg) {    // Check if masking is enabled for this background.
                self.show_bg_pixel(bg, x)
            } else {
                true
            }
        } else {
            false
        }
    }

    // Returns true if the pixel should be shown for the objects on the main screen.
    pub fn show_obj_pixel_main(&self, x: u8) -> bool {
        if self.enable_obj_main() {    // Check if objects are enabled for the main screen.
            if self.enable_masking_obj_main() {    // Check if masking is enabled for objects.
                self.show_obj_pixel(x)
            } else {
                true
            }
        } else {
            false
        }
    }

    // Returns true if the pixel should be shown for the bg specified on the sub screen.
    pub fn show_bg_pixel_sub(&self, bg: usize, x: u8) -> bool {
        if self.enable_bg_sub(bg) {    // Check if this bg is enabled for the sub screen.
            if self.enable_masking_bg_sub(bg) {    // Check if masking is enabled for this background.
                self.show_bg_pixel(bg, x)
            } else {
                true
            }
        } else {
            false
        }
    }

    // Returns true if the pixel should be shown for the objects on the sub screen.
    pub fn show_obj_pixel_sub(&self, x: u8) -> bool {
        if self.enable_obj_sub() {    // Check if objects are enabled for the sub screen.
            if self.enable_masking_obj_sub() {    // Check if masking is enabled for objects.
                self.show_obj_pixel(x)
            } else {
                true
            }
        } else {
            false
        }
    }
}

// internal helpers
impl WindowRegisters {
    // Returns true if the bg pixel specified should be shown through the window mask
    fn show_bg_pixel(&self, bg: usize, x: u8) -> bool {
        let enable_1 = self.enable_window_1_bg(bg);
        let enable_2 = self.enable_window_2_bg(bg);
        match (enable_1, enable_2) {
            (true, true) => {   // Use op to combine
                let win_1 = self.test_inside_window_1(x) != self.invert_window_1_bg(bg);
                let win_2 = self.test_inside_window_2(x) != self.invert_window_2_bg(bg);
                let op = self.window_op_bg(bg);
                do_window_op(op, win_1, win_2)
            },
            (true, false) => {
                self.test_inside_window_1(x) != self.invert_window_1_bg(bg)
            },
            (false, true) => {  // Just use window 2
                self.test_inside_window_2(x) != self.invert_window_2_bg(bg)
            },
            (false, false) => { // No windows enabled for bg.
                true
            }
        }
    }

    // Returns true if the obj pixel specified should be shown through the window mask
    fn show_obj_pixel(&self, x: u8) -> bool {
        let enable_1 = self.enable_window_1_obj();
        let enable_2 = self.enable_window_2_obj();
        match (enable_1, enable_2) {
            (true, true) => {   // Use op to combine
                let win_1 = self.test_inside_window_1(x) != self.invert_window_1_obj();
                let win_2 = self.test_inside_window_2(x) != self.invert_window_2_obj();
                let op = self.window_op_obj();
                do_window_op(op, win_1, win_2)
            },
            (true, false) => {  // Just use window 1
                self.test_inside_window_1(x) != self.invert_window_1_obj()
            },
            (false, true) => {  // Just use window 2
                self.test_inside_window_2(x) != self.invert_window_2_obj()
            },
            (false, false) => { // No windows enabled for objects
                true
            }
        }
    }

    // Returns true if layer is enabled for the screen
    fn enable_bg_main(&self, bg: usize) -> bool {
        match bg {
            0 => self.main_screen_desg.contains(LayerDesignation::BG1),
            1 => self.main_screen_desg.contains(LayerDesignation::BG2),
            2 => self.main_screen_desg.contains(LayerDesignation::BG3),
            3 => self.main_screen_desg.contains(LayerDesignation::BG4),
            _ => unreachable!()
        }
    }
    fn enable_obj_main(&self) -> bool {
        self.main_screen_desg.contains(LayerDesignation::OBJ)
    }
    fn enable_bg_sub(&self, bg: usize) -> bool {
        match bg {
            0 => self.sub_screen_desg.contains(LayerDesignation::BG1),
            1 => self.sub_screen_desg.contains(LayerDesignation::BG2),
            2 => self.sub_screen_desg.contains(LayerDesignation::BG3),
            3 => self.sub_screen_desg.contains(LayerDesignation::BG4),
            _ => unreachable!()
        }
    }
    fn enable_obj_sub(&self) -> bool {
        self.sub_screen_desg.contains(LayerDesignation::OBJ)
    }

    // Returns true if masking is enabled for the BG
    fn enable_masking_bg_main(&self, bg: usize) -> bool {
        match bg {
            0 => self.main_mask_desg.contains(LayerDesignation::BG1),
            1 => self.main_mask_desg.contains(LayerDesignation::BG2),
            2 => self.main_mask_desg.contains(LayerDesignation::BG3),
            3 => self.main_mask_desg.contains(LayerDesignation::BG4),
            _ => unreachable!()
        }
    }
    // Returns true if masking is enabled for objects
    fn enable_masking_obj_main(&self) -> bool {
        self.main_mask_desg.contains(LayerDesignation::OBJ)
    }

    // Returns true if masking is enabled for the BG
    fn enable_masking_bg_sub(&self, bg: usize) -> bool {
        match bg {
            0 => self.sub_mask_desg.contains(LayerDesignation::BG1),
            1 => self.sub_mask_desg.contains(LayerDesignation::BG2),
            2 => self.sub_mask_desg.contains(LayerDesignation::BG3),
            3 => self.sub_mask_desg.contains(LayerDesignation::BG4),
            _ => unreachable!()
        }
    }
    // Returns true if masking is enabled for objects
    fn enable_masking_obj_sub(&self) -> bool {
        self.sub_mask_desg.contains(LayerDesignation::OBJ)
    }

    // Returns true if window 1 is enabled for layer
    fn enable_window_1_bg(&self, bg: usize) -> bool {
        match bg {
            0 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_ENABLE_LO),
            1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_ENABLE_HI),
            2 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_ENABLE_LO),
            3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_ENABLE_HI),
            _ => unreachable!()
        }
    }
    fn enable_window_1_obj(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_ENABLE_LO)
    }
    fn enable_window_1_col(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_ENABLE_HI)
    }

    // Returns true if window 2 is enabled for layer
    fn enable_window_2_bg(&self, bg: usize) -> bool {
        match bg {
            0 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_ENABLE_LO),
            1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_ENABLE_HI),
            2 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_ENABLE_LO),
            3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_ENABLE_HI),
            _ => unreachable!()
        }
    }
    fn enable_window_2_obj(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_2_ENABLE_LO)
    }
    fn enable_window_2_col(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_2_ENABLE_HI)
    }

    // Returns true if window 1 should be inverted for layer
    fn invert_window_1_bg(&self, bg: usize) -> bool {
        match bg {
            0 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_INVERT_LO),
            1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_INVERT_HI),
            2 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_INVERT_LO),
            3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_INVERT_HI),
            _ => unreachable!()
        }
    }
    fn invert_window_1_obj(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_INVERT_LO)
    }
    fn invert_window_1_col(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_INVERT_HI)
    }

    // Returns true if window 2 should be inverted for layer
    fn invert_window_2_bg(&self, bg: usize) -> bool {
        match bg {
            0 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_INVERT_LO),
            1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_INVERT_HI),
            2 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_INVERT_LO),
            3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_INVERT_HI),
            _ => unreachable!()
        }
    }
    fn invert_window_2_obj(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_2_INVERT_LO)
    }
    fn invert_window_2_col(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_2_INVERT_HI)
    }

    // Returns true if the pixel specified is inside the window range.
    fn test_inside_window_1(&self, x: u8) -> bool {
        (x <= self.window_1_right) && (x >= self.window_1_left)
    }
    fn test_inside_window_2(&self, x: u8) -> bool {
        (x <= self.window_2_right) && (x >= self.window_2_left)
    }

    // Returns the window operation for the layer
    fn window_op_bg(&self, bg: usize) -> u8 {
        match bg {
            0 => (self.mask_logic_bg & BGMaskLogic::BG_1_OP).bits(),
            1 => (self.mask_logic_bg & BGMaskLogic::BG_2_OP).bits() >> 2,
            2 => (self.mask_logic_bg & BGMaskLogic::BG_3_OP).bits() >> 4,
            3 => (self.mask_logic_bg & BGMaskLogic::BG_4_OP).bits() >> 6,
            _ => unreachable!()
        }
    }
    fn window_op_obj(&self) -> u8 {
        (self.mask_logic_obj_col & ObjColMaskLogic::OBJ_OP).bits()
    }
    fn window_op_col(&self) -> u8 {
        (self.mask_logic_obj_col & ObjColMaskLogic::COL_OP).bits() >> 2
    }
}

// Does the window operation
// TODO: make this a type?
fn do_window_op(op: u8, win_1: bool, win_2: bool) -> bool {
    match op {
        MASK_LOGIC_OR   => win_1 || win_2,
        MASK_LOGIC_AND  => win_1 && win_2,
        MASK_LOGIC_XOR  => win_1 != win_2,
        MASK_LOGIC_XNOR => win_1 == win_2,
        _ => unreachable!()
    }
}