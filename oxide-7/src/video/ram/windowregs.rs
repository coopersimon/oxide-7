// Video registers for colour math and window settings.
use bitflags::bitflags;

use crate::video::{
    BG,
    render::Colour
};

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
    struct ColourAddSelect: u8 {
        const CLIP_TO_BLACK = bits![7, 6];
        const PREVENT       = bits![5, 4];
        const USE_SUB       = bit!(1);
        const DIRECT_COL    = bit!(0);
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

bitflags! {
    #[derive(Default)]
    struct VideoSelect: u8 {
        const MODE_7_EXT_BG     = bit!(6);
        const PSEUDO_HIRES      = bit!(3);
        const OVERSCAN          = bit!(2);
        const OBJ_INTERLACE     = bit!(1);
        const SCREEN_INTERLACE  = bit!(0);
    }
}

// Used as input to some window reg methods.
#[derive(Clone, Copy)]
pub enum Screen {
    Main,
    Sub
}

// Different operations used to combine masking windows.
#[derive(Clone, Copy)]
enum WindowOp {
    OR,
    AND,
    XOR,
    XNOR
}

impl From<u8> for WindowOp {
    // Should be from a 2-bit value.
    fn from(val: u8) -> Self {
        const MASK_LOGIC_OR: u8 = 0;
        const MASK_LOGIC_AND: u8 = 1;
        const MASK_LOGIC_XOR: u8 = 2;
        const MASK_LOGIC_XNOR: u8 = 3;

        match val {
            MASK_LOGIC_OR   => Self::OR,
            MASK_LOGIC_AND  => Self::AND,
            MASK_LOGIC_XOR  => Self::XOR,
            MASK_LOGIC_XNOR => Self::XNOR,
            _ => unreachable!()
        }
    }
}

impl WindowOp {
    // Use the op to combine the boolean variables.
    fn combine(self, win_1: bool, win_2: bool) -> bool {
        use WindowOp::*;
        match self {
            OR   => win_1 || win_2,
            AND  => win_1 && win_2,
            XOR  => win_1 != win_2,
            XNOR => win_1 == win_2,
        }
    }
}

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

    colour_add_select:  ColourAddSelect,
    colour_math_desg:   ColourMathDesignation,

    fixed_colour:       Colour,
    video_select:       VideoSelect,
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

            colour_add_select:  ColourAddSelect::default(),
            colour_math_desg:   ColourMathDesignation::default(),

            fixed_colour:       Colour::zero(),
            video_select:       VideoSelect::default(),
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
        self.colour_add_select = ColourAddSelect::from_bits_truncate(data);
        //println!("Colour add sel: {:X}", data);
    }

    pub fn set_colour_math_desg(&mut self, data: u8) {
        self.colour_math_desg = ColourMathDesignation::from_bits_truncate(data);
    }

    pub fn set_fixed_colour(&mut self, data: u8) {
        const FIXED_COLOUR_B_BIT: u8 = 7;
        const FIXED_COLOUR_G_BIT: u8 = 6;
        const FIXED_COLOUR_R_BIT: u8 = 5;
        const MAX_COLOUR: u8 = 0x1F;

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

    pub fn set_video_select(&mut self, data: u8) {
        self.video_select = VideoSelect::from_bits_truncate(data);
        //println!("Video sel: {:X}", data);
        /*if self.video_select.contains(VideoSelect::SCREEN_INTERLACE) {
            println!("Interlace enabled!")
        }*/
    }

    // Getters - renderer side

    // Sets the input bool slice to false if the associated pixel shouldn't be shown.
    // Assumes an input of true bools.
    pub fn bg_window(&self, bg: BG, screen: Screen, window_line: &mut [bool]) {
        if self.enable_bg(bg, screen) {                 // Check if this bg is enabled for the screen.
            if self.enable_masking_bg(bg, screen) {     // Check if masking is enabled for this background.
                for (x, w) in window_line.iter_mut().enumerate() {
                    *w = !self.mask_bg_pixel(bg, x as u8);
                }
            }
        } else {
            for w in window_line.iter_mut() {
                *w = false;
            }
        }
    }

    // Returns true if the pixel should be shown for the objects on a screen.
    pub fn show_obj_pixel(&self, screen: Screen, x: u8) -> bool {
        if self.enable_obj(screen) {    // Check if objects are enabled for the screen.
            if self.enable_masking_obj(screen) {    // Check if masking is enabled for objects.
                !self.mask_obj_pixel(x)
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn get_fixed_colour(&self) -> Colour {
        self.fixed_colour
    }

    // Returns true if subscreen should be used, false if fixed colour should be used.
    pub fn use_subscreen(&self) -> bool {
        self.colour_add_select.contains(ColourAddSelect::USE_SUB)
    }

    // Returns true if direct colour mode should be used for modes 3, 4 and 7 8bpp backgrounds.
    pub fn use_direct_colour(&self) -> bool {
        self.colour_add_select.contains(ColourAddSelect::DIRECT_COL)
    }

    // Returns true if pseudo hi-res mode is on.
    pub fn use_pseudo_hires(&self) -> bool {
        self.video_select.contains(VideoSelect::PSEUDO_HIRES)
    }

    // Returns true if the mode 7 extra background should be used.
    pub fn use_ext_bg(&self) -> bool {
        self.video_select.contains(VideoSelect::MODE_7_EXT_BG)
    }

    // Combine colours.
    pub fn calc_colour_math_bg(&self, main: Colour, sub: Option<Colour>, bg: BG, x: u8) -> Colour {
        if self.enable_bg_colour_math(bg) {
            self.do_colour_math(main, sub, x)
        } else {
            main
        }
    }

    // Combine colours.
    pub fn calc_colour_math_obj(&self, main: Colour, sub: Option<Colour>, x: u8) -> Colour {
        if self.enable_obj_colour_math() {
            self.do_colour_math(main, sub, x)
        } else {
            main
        }
    }

    // Combine colours.
    pub fn calc_colour_math_backdrop(&self, main: Colour, sub: Option<Colour>, x: u8) -> Colour {
        if self.enable_backdrop_colour_math() {
            self.do_colour_math(main, sub, x)
        } else {
            main
        }
    }
}

// internal helpers
impl WindowRegisters {
    // Returns true if the bg pixel specified is inside the mask.
    fn mask_bg_pixel(&self, bg: BG, x: u8) -> bool {
        let enable_1 = self.enable_window_1_bg(bg);
        let enable_2 = self.enable_window_2_bg(bg);
        match (enable_1, enable_2) {
            (true, true) => {   // Use op to combine
                let win_1 = self.test_inside_window_1(x) != self.invert_window_1_bg(bg);
                let win_2 = self.test_inside_window_2(x) != self.invert_window_2_bg(bg);
                self.window_op_bg(bg).combine(win_1, win_2)
            },
            (true, false) => {  // Just use window 1
                self.test_inside_window_1(x) != self.invert_window_1_bg(bg)
            },
            (false, true) => {  // Just use window 2
                self.test_inside_window_2(x) != self.invert_window_2_bg(bg)
            },
            (false, false) => { // No windows enabled for bg.
                false
            }
        }
    }

    // Returns true if the obj pixel specified is inside the mask.
    fn mask_obj_pixel(&self, x: u8) -> bool {
        let enable_1 = self.enable_window_1_obj();
        let enable_2 = self.enable_window_2_obj();
        match (enable_1, enable_2) {
            (true, true) => {   // Use op to combine
                let win_1 = self.test_inside_window_1(x) != self.invert_window_1_obj();
                let win_2 = self.test_inside_window_2(x) != self.invert_window_2_obj();
                self.window_op_obj().combine(win_1, win_2)
            },
            (true, false) => {  // Just use window 1
                self.test_inside_window_1(x) != self.invert_window_1_obj()
            },
            (false, true) => {  // Just use window 2
                self.test_inside_window_2(x) != self.invert_window_2_obj()
            },
            (false, false) => { // No windows enabled for objects
                false
            }
        }
    }

    // Returns true if the colour math pixel specified is inside the colour mask.
    fn col_window_pixel(&self, x: u8) -> bool {
        let enable_1 = self.enable_window_1_col();
        let enable_2 = self.enable_window_2_col();
        match (enable_1, enable_2) {
            (true, true) => {   // Use op to combine
                let win_1 = self.test_inside_window_1(x) != self.invert_window_1_col();
                let win_2 = self.test_inside_window_2(x) != self.invert_window_2_col();
                self.window_op_col().combine(win_1, win_2)
            },
            (true, false) => {  // Just use window 1
                self.test_inside_window_1(x) != self.invert_window_1_col()
            },
            (false, true) => {  // Just use window 2
                self.test_inside_window_2(x) != self.invert_window_2_col()
            },
            (false, false) => { // No windows enabled for colour
                false
            }
        }
    }

    // Returns true if layer is enabled for the screen
    fn enable_bg(&self, bg: BG, screen: Screen) -> bool {
        match screen {
            Screen::Main => match bg {
                BG::_1 => self.main_screen_desg.contains(LayerDesignation::BG1),
                BG::_2 => self.main_screen_desg.contains(LayerDesignation::BG2),
                BG::_3 => self.main_screen_desg.contains(LayerDesignation::BG3),
                BG::_4 => self.main_screen_desg.contains(LayerDesignation::BG4),
            },
            Screen::Sub => match bg {
                BG::_1 => self.sub_screen_desg.contains(LayerDesignation::BG1),
                BG::_2 => self.sub_screen_desg.contains(LayerDesignation::BG2),
                BG::_3 => self.sub_screen_desg.contains(LayerDesignation::BG3),
                BG::_4 => self.sub_screen_desg.contains(LayerDesignation::BG4),
            }
        }
    }
    fn enable_obj(&self, screen: Screen) -> bool {
        match screen {
            Screen::Main => self.main_screen_desg.contains(LayerDesignation::OBJ),
            Screen::Sub => self.sub_screen_desg.contains(LayerDesignation::OBJ)
        }
    }

    // Returns true if masking is enabled for the BG
    fn enable_masking_bg(&self, bg: BG, screen: Screen) -> bool {
        match screen {
            Screen::Main => match bg {
                BG::_1 => self.main_mask_desg.contains(LayerDesignation::BG1),
                BG::_2 => self.main_mask_desg.contains(LayerDesignation::BG2),
                BG::_3 => self.main_mask_desg.contains(LayerDesignation::BG3),
                BG::_4 => self.main_mask_desg.contains(LayerDesignation::BG4),
            },
            Screen::Sub => match bg {
                BG::_1 => self.sub_mask_desg.contains(LayerDesignation::BG1),
                BG::_2 => self.sub_mask_desg.contains(LayerDesignation::BG2),
                BG::_3 => self.sub_mask_desg.contains(LayerDesignation::BG3),
                BG::_4 => self.sub_mask_desg.contains(LayerDesignation::BG4),
            }
        }
    }
    // Returns true if masking is enabled for objects
    fn enable_masking_obj(&self, screen: Screen) -> bool {
        match screen {
            Screen::Main => self.main_mask_desg.contains(LayerDesignation::OBJ),
            Screen::Sub => self.sub_mask_desg.contains(LayerDesignation::OBJ)
        }
    }

    // Returns true if window 1 is enabled for layer
    fn enable_window_1_bg(&self, bg: BG) -> bool {
        match bg {
            BG::_1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_ENABLE_LO),
            BG::_2 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_ENABLE_HI),
            BG::_3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_ENABLE_LO),
            BG::_4 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_ENABLE_HI),
        }
    }
    fn enable_window_1_obj(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_ENABLE_LO)
    }
    fn enable_window_1_col(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_ENABLE_HI)
    }

    // Returns true if window 2 is enabled for layer
    fn enable_window_2_bg(&self, bg: BG) -> bool {
        match bg {
            BG::_1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_ENABLE_LO),
            BG::_2 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_ENABLE_HI),
            BG::_3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_ENABLE_LO),
            BG::_4 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_ENABLE_HI),
        }
    }
    fn enable_window_2_obj(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_2_ENABLE_LO)
    }
    fn enable_window_2_col(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_2_ENABLE_HI)
    }

    // Returns true if window 1 should be inverted for layer
    fn invert_window_1_bg(&self, bg: BG) -> bool {
        match bg {
            BG::_1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_INVERT_LO),
            BG::_2 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_1_INVERT_HI),
            BG::_3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_INVERT_LO),
            BG::_4 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_1_INVERT_HI),
        }
    }
    fn invert_window_1_obj(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_INVERT_LO)
    }
    fn invert_window_1_col(&self) -> bool {
        self.mask_obj_col.contains(WindowMaskSettings::WINDOW_1_INVERT_HI)
    }

    // Returns true if window 2 should be inverted for layer
    fn invert_window_2_bg(&self, bg: BG) -> bool {
        match bg {
            BG::_1 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_INVERT_LO),
            BG::_2 => self.mask_bg1_2.contains(WindowMaskSettings::WINDOW_2_INVERT_HI),
            BG::_3 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_INVERT_LO),
            BG::_4 => self.mask_bg3_4.contains(WindowMaskSettings::WINDOW_2_INVERT_HI),
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
    fn window_op_bg(&self, bg: BG) -> WindowOp {
        match bg {
            BG::_1 => (self.mask_logic_bg & BGMaskLogic::BG_1_OP).bits(),
            BG::_2 => (self.mask_logic_bg & BGMaskLogic::BG_2_OP).bits() >> 2,
            BG::_3 => (self.mask_logic_bg & BGMaskLogic::BG_3_OP).bits() >> 4,
            BG::_4 => (self.mask_logic_bg & BGMaskLogic::BG_4_OP).bits() >> 6,
        }.into()
    }
    fn window_op_obj(&self) -> WindowOp {
        (self.mask_logic_obj_col & ObjColMaskLogic::OBJ_OP).bits().into()
    }
    fn window_op_col(&self) -> WindowOp {
        ((self.mask_logic_obj_col & ObjColMaskLogic::COL_OP).bits() >> 2).into()
    }

    fn enable_bg_colour_math(&self, bg: BG) -> bool {
        match bg {
            BG::_1 => self.colour_math_desg.contains(ColourMathDesignation::BG1),
            BG::_2 => self.colour_math_desg.contains(ColourMathDesignation::BG2),
            BG::_3 => self.colour_math_desg.contains(ColourMathDesignation::BG3),
            BG::_4 => self.colour_math_desg.contains(ColourMathDesignation::BG4),
        }
    }

    fn enable_obj_colour_math(&self) -> bool {
        self.colour_math_desg.contains(ColourMathDesignation::OBJ)
    }

    fn enable_backdrop_colour_math(&self) -> bool {
        self.colour_math_desg.contains(ColourMathDesignation::BACKDROP)
    }

    // Returns true if main screen pixel should be clipped to black.
    fn clip_to_black(&self, x: u8) -> bool {
        const CLIP_NEVER: u8 = 0 << 6;
        const CLIP_OUTSIDE: u8 = 1 << 6;
        const CLIP_INSIDE: u8 = 2 << 6;
        const CLIP_ALWAYS: u8 = 3 << 6;
    
        match (self.colour_add_select & ColourAddSelect::CLIP_TO_BLACK).bits() {
            CLIP_NEVER => false,
            CLIP_OUTSIDE => !self.col_window_pixel(x),
            CLIP_INSIDE => self.col_window_pixel(x),
            CLIP_ALWAYS => true,
            _ => unreachable!()
        }
    }

    // Returns true if colour math should happen.
    fn should_do_colour_math(&self, x: u8) -> bool {
        const PREVENT_NEVER: u8 = 0 << 4;
        const PREVENT_OUTSIDE: u8 = 1 << 4;
        const PREVENT_INSIDE: u8 = 2 << 4;
        const PREVENT_ALWAYS: u8 = 3 << 4;
    
        match (self.colour_add_select & ColourAddSelect::PREVENT).bits() {
            PREVENT_NEVER => true,
            PREVENT_OUTSIDE => self.col_window_pixel(x),
            PREVENT_INSIDE => !self.col_window_pixel(x),
            PREVENT_ALWAYS => false,
            _ => unreachable!()
        }
    }

    fn do_colour_math(&self, main: Colour, sub: Option<Colour>, x: u8) -> Colour {
        let clip_colour = self.clip_to_black(x);
        let main_col = if clip_colour {Colour::zero()} else {main};
        let sub_col = sub.unwrap_or(self.get_fixed_colour());
        if self.should_do_colour_math(x) {
            let (r, g, b) = if !self.colour_math_desg.contains(ColourMathDesignation::ADD_SUB) {
                let i_r = main_col.r as u16 + sub_col.r as u16;
                let i_g = main_col.g as u16 + sub_col.g as u16;
                let i_b = main_col.b as u16 + sub_col.b as u16;
                (i_r, i_g, i_b)
            } else {
                let i_r = main_col.r as i16 - sub_col.r as i16;
                let i_g = main_col.g as i16 - sub_col.g as i16;
                let i_b = main_col.b as i16 - sub_col.b as i16;
                (
                    if i_r < 0 {0} else {i_r as u16},
                    if i_g < 0 {0} else {i_g as u16},
                    if i_b < 0 {0} else {i_b as u16},
                )
            };

            let sub_backdrop = self.colour_add_select.contains(ColourAddSelect::USE_SUB) && sub.is_none();
            if self.colour_math_desg.contains(ColourMathDesignation::HALF) &&
                !clip_colour &&
                !sub_backdrop {
                Colour::new(lo!(r >> 1), lo!(g >> 1), lo!(b >> 1))
            } else {
                Colour::new(
                    if r > 0xFF {0xFF} else {r as u8},
                    if g > 0xFF {0xFF} else {g as u8},
                    if b > 0xFF {0xFF} else {b as u8},
                )
            }
        } else {
            main_col
        }
    }
}
