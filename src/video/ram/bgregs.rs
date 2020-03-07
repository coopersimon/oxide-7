// Other video registers, for BG settings

use bitflags::bitflags;
use fixed::types::I8F8;

use std::collections::BTreeSet;

use crate::video::BG;

bitflags! {
    #[derive(Default)]
    pub struct ScreenDisplay: u8 {
        const F_BLANK       = bit!(7);
        const BRIGHTNESS    = bits![3, 2, 1, 0];
    }
}

bitflags! {
    #[derive(Default)]
    pub struct ObjectSettings: u8 {
        const SIZE      = bits![7, 6, 5];
        const SELECT    = bits![4, 3];
        const BASE      = bits![2, 1, 0];
    }
}

bitflags! {
    #[derive(Default)]
    pub struct BGMode: u8 {
        const BG4_TILE_SIZE = bit!(7);
        const BG3_TILE_SIZE = bit!(6);
        const BG2_TILE_SIZE = bit!(5);
        const BG1_TILE_SIZE = bit!(4);
        const BG3_PRIORITY  = bit!(3);
        const MODE          = bits![2, 1, 0];
    }
}

// BG Register bits.
bitflags! {
    #[derive(Default)]
    pub struct BGReg: u8 {
        const ADDR      = bits![7, 6, 5, 4, 3, 2];
        const MIRROR_Y  = bit!(1);
        const MIRROR_X  = bit!(0);
    }
}

// Combination of mirror X and Y.
pub enum MapMirror {
    None    = 0,
    X       = 1,
    Y       = 2,
    Both    = 3
}

impl From<BGReg> for MapMirror {
    fn from(val: BGReg) -> Self {
        match (val & (BGReg::MIRROR_Y | BGReg::MIRROR_X)).bits() {
            0 => MapMirror::None,
            1 => MapMirror::X,
            2 => MapMirror::Y,
            3 => MapMirror::Both,
            _ => unreachable!()
        }
    }
}

bitflags! {
    #[derive(Default)]
    struct Mosaic: u8 {
        const PIXEL_SIZE = bits![7, 6, 5, 4];
        const BG4_ENABLE = bit!(3);
        const BG3_ENABLE = bit!(2);
        const BG2_ENABLE = bit!(1);
        const BG1_ENABLE = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    struct Mode7Settings: u8 {
        const FIELD_SIZE    = bit!(7);
        const EMPTY_FILL    = bit!(6);
        const FLIP_Y        = bit!(1);
        const FLIP_X        = bit!(0);
    }
}

pub enum Mode7Extend {
    Repeat,
    Transparent,
    Clamp
}

impl From<Mode7Settings> for Mode7Extend {
    fn from(val: Mode7Settings) -> Mode7Extend {
        if !val.contains(Mode7Settings::FIELD_SIZE) {
            Mode7Extend::Repeat
        } else if val.contains(Mode7Settings::EMPTY_FILL) {
            Mode7Extend::Clamp
        } else {
            Mode7Extend::Transparent
        }
    }
}

const VRAM_END_ADDR: u32 = 64 * 1024;
const PATTERN_MAX_HEIGHT: u32 = 64;
const BG_SCROLL_MASK: u16 = 0x3FF;
const MODE_7_SCROLL_MASK: u16 = 0x1FFF;

// Sub-map size (32x32 tiles)
const SUB_MAP_LEN: u16 = 32;
const SUB_MAP_SIZE: u16 = SUB_MAP_LEN * SUB_MAP_LEN * 2;

pub struct Registers {

        screen_display:     ScreenDisplay,
        object_settings:    ObjectSettings,
        bg_mode:            BGMode,
        mosaic_settings:    Mosaic,

        bg1_settings:       BGReg,
        bg2_settings:       BGReg,
        bg3_settings:       BGReg,
        bg4_settings:       BGReg,

        bg12_char_addr:     u8,
        bg34_char_addr:     u8,

        bg1_scroll_x:       u16,
        bg1_scroll_y:       u16,
        bg2_scroll_x:       u16,
        bg2_scroll_y:       u16,
        bg3_scroll_x:       u16,
        bg3_scroll_y:       u16,
        bg4_scroll_x:       u16,
        bg4_scroll_y:       u16,

        mode7_settings:     Mode7Settings,
        mode7_prev:         u8, // The last written value to a mode 7 register.
        mode7_scroll_x:     u16,
        mode7_scroll_y:     u16,
        mode7_matrix_a:     u16,
        mode7_matrix_b:     u16,
        mode7_matrix_c:     u16,
        mode7_matrix_d:     u16,
        mode7_centre_x:     u16,
        mode7_centre_y:     u16,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            screen_display:     ScreenDisplay::default(),
            object_settings:    ObjectSettings::default(),
            bg_mode:            BGMode::default(),
            mosaic_settings:    Mosaic::default(),

            bg1_settings:       BGReg::default(),
            bg2_settings:       BGReg::default(),
            bg3_settings:       BGReg::default(),
            bg4_settings:       BGReg::default(),

            bg12_char_addr:     0,
            bg34_char_addr:     0,

            bg1_scroll_x:       0,
            bg1_scroll_y:       0,
            bg2_scroll_x:       0,
            bg2_scroll_y:       0,
            bg3_scroll_x:       0,
            bg3_scroll_y:       0,
            bg4_scroll_x:       0,
            bg4_scroll_y:       0,

            mode7_settings:     Mode7Settings::default(),
            mode7_prev:         0,
            mode7_scroll_x:     0,
            mode7_scroll_y:     0,
            mode7_matrix_a:     0,
            mode7_matrix_b:     0,
            mode7_matrix_c:     0,
            mode7_matrix_d:     0,
            mode7_centre_x:     0,
            mode7_centre_y:     0,
        }
    }

    // Setters (CPU side)
    pub fn set_screen_display(&mut self, data: u8) {
        self.screen_display = ScreenDisplay::from_bits_truncate(data);
    }

    pub fn set_object_settings(&mut self, data: u8) {
        self.object_settings = ObjectSettings::from_bits_truncate(data);
    }

    pub fn set_bg_mode(&mut self, data: u8) {
        self.bg_mode = BGMode::from_bits_truncate(data);
    }

    pub fn set_mosaic(&mut self, data: u8) {
        self.mosaic_settings = Mosaic::from_bits_truncate(data);
    }

    pub fn set_bg1_scroll_x(&mut self, data: u8) {
        self.bg1_scroll_x = make16!(data, hi!(self.bg1_scroll_x));
        self.mode7_scroll_x = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    pub fn set_bg1_scroll_y(&mut self, data: u8) {
        self.bg1_scroll_y = make16!(data, hi!(self.bg1_scroll_y));
        self.mode7_scroll_y = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    pub fn set_bg2_scroll_x(&mut self, data: u8) {
        self.bg2_scroll_x = make16!(data, hi!(self.bg2_scroll_x));
    }

    pub fn set_bg2_scroll_y(&mut self, data: u8) {
        self.bg2_scroll_y = make16!(data, hi!(self.bg2_scroll_y));
    }

    pub fn set_bg3_scroll_x(&mut self, data: u8) {
        self.bg3_scroll_x = make16!(data, hi!(self.bg3_scroll_x));
    }

    pub fn set_bg3_scroll_y(&mut self, data: u8) {
        self.bg3_scroll_y = make16!(data, hi!(self.bg3_scroll_y));
    }

    pub fn set_bg4_scroll_x(&mut self, data: u8) {
        self.bg4_scroll_x = make16!(data, hi!(self.bg4_scroll_x));
    }

    pub fn set_bg4_scroll_y(&mut self, data: u8) {
        self.bg4_scroll_y = make16!(data, hi!(self.bg4_scroll_y));
    }

    pub fn set_bg1_settings(&mut self, data: u8) {
        self.bg1_settings = BGReg::from_bits_truncate(data);
    }

    pub fn set_bg2_settings(&mut self, data: u8) {
        self.bg2_settings = BGReg::from_bits_truncate(data);
    }

    pub fn set_bg3_settings(&mut self, data: u8) {
        self.bg3_settings = BGReg::from_bits_truncate(data);
    }

    pub fn set_bg4_settings(&mut self, data: u8) {
        self.bg4_settings = BGReg::from_bits_truncate(data);
    }

    pub fn set_bg12_char_addr(&mut self, data: u8) {
        self.bg12_char_addr = data;
    }

    pub fn set_bg34_char_addr(&mut self, data: u8) {
        self.bg34_char_addr = data;
    }
    
    pub fn set_mode7_settings(&mut self, data: u8) {
        self.mode7_settings = Mode7Settings::from_bits_truncate(data);
    }

    pub fn set_mode7_matrix_a(&mut self, data: u8) {
        self.mode7_matrix_a = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    pub fn set_mode7_matrix_b(&mut self, data: u8) {
        self.mode7_matrix_b = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    pub fn set_mode7_matrix_c(&mut self, data: u8) {
        self.mode7_matrix_c = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    pub fn set_mode7_matrix_d(&mut self, data: u8) {
        self.mode7_matrix_d = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    pub fn set_mode7_centre_x(&mut self, data: u8) {
        self.mode7_centre_x = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    pub fn set_mode7_centre_y(&mut self, data: u8) {
        self.mode7_centre_y = make16!(data, self.mode7_prev);
        self.mode7_prev = data;
    }

    // Getters (CPU side)
    pub fn read_mult_result_lo(&self) -> u8 {
        let result = (self.mode7_matrix_a as i32) * (hi!(self.mode7_matrix_b) as i32);
        lo24!(result as u32, u8)
    }

    pub fn read_mult_result_mid(&self) -> u8 {
        let result = (self.mode7_matrix_a as i32) * (hi!(self.mode7_matrix_b) as i32);
        mid24!(result as u32)
    }

    pub fn read_mult_result_hi(&self) -> u8 {
        let result = (self.mode7_matrix_a as i32) * (hi!(self.mode7_matrix_b) as i32);
        hi24!(result as u32)
    }
}

// Getters for the renderer.
impl Registers {

    pub fn in_fblank(&self) -> bool {
        self.screen_display.contains(ScreenDisplay::F_BLANK)
    }

    pub fn get_brightness(&self) -> u8 {
        (self.screen_display & ScreenDisplay::BRIGHTNESS).bits()
    }

    pub fn get_mode(&self) -> u8 {
        (self.bg_mode & BGMode::MODE).bits()
    }

    // Modes 5 and 6 always use 16-pixel wide tiles.
    pub fn use_wide_tiles(&self) -> bool {
        match self.get_mode() {
            5 | 6 => true,
            _ => false
        }
    }

    pub fn get_bg3_priority(&self) -> bool {
        self.bg_mode.contains(BGMode::BG3_PRIORITY)
    }

    pub fn get_object_settings(&self) -> u8 {
        self.object_settings.bits()
    }

    pub fn obj0_pattern_addr(&self) -> u16 {
        let base = (self.object_settings & ObjectSettings::BASE).bits() as u16;
        base << 14
    }

    pub fn objn_pattern_addr(&self) -> u16 {
        let base = (self.object_settings & ObjectSettings::BASE).bits() as u16;
        let table = (((self.object_settings & ObjectSettings::SELECT).bits() as u16) >> 3) + 1;
        (base << 14) + (table << 13)
    }

    pub fn get_bg_settings(&self, bg: BG) -> BGReg {
        match bg {
            BG::_1 => self.bg1_settings,
            BG::_2 => self.bg2_settings,
            BG::_3 => self.bg3_settings,
            BG::_4 => self.bg4_settings,
        }
    }

    // TODO: use less magic numbers in the following.
    pub fn bg_pattern_addr(&self, bg: BG) -> u16 {
        match bg {
            BG::_1 => ((self.bg12_char_addr & 0xF) as u16) << 13,
            BG::_2 => ((self.bg12_char_addr & 0xF0) as u16) << 9,
            BG::_3 => ((self.bg34_char_addr & 0xF) as u16) << 13,
            BG::_4 => ((self.bg34_char_addr & 0xF0) as u16) << 9,
        }
    }

    pub fn bg_map_addr(&self, bg: BG) -> u16 {
        match bg {
            BG::_1 => ((self.bg1_settings & BGReg::ADDR).bits() as u16) << 9,
            BG::_2 => ((self.bg2_settings & BGReg::ADDR).bits() as u16) << 9,
            BG::_3 => ((self.bg3_settings & BGReg::ADDR).bits() as u16) << 9,
            BG::_4 => ((self.bg4_settings & BGReg::ADDR).bits() as u16) << 9,
        }
    }

    pub fn bg_size_mask(&self, bg: BG) -> (usize, usize) {
        let large_tiles = self.bg_large_tiles(bg);
        let map_mirror = MapMirror::from(self.get_bg_settings(bg));
        match (map_mirror, large_tiles) {
            (MapMirror::None, false)    => (255, 255),
            (MapMirror::X, false)       => (511, 255),
            (MapMirror::Y, false)       => (255, 511),
            (MapMirror::Both, false)    => (511, 511),
            (MapMirror::None, true)     => (511, 511),
            (MapMirror::X, true)        => (1023, 511),
            (MapMirror::Y, true)        => (511, 1023),
            (MapMirror::Both, true)     => (1023, 1023),
        }
    }

    // Get size of the background in tiles.
    pub fn bg_size_tiles(&self, bg: BG) -> (usize, usize) {
        let map_mirror = MapMirror::from(self.get_bg_settings(bg));
        match map_mirror {
            MapMirror::None => (32, 32),
            MapMirror::X    => (64, 32),
            MapMirror::Y    => (32, 64),
            MapMirror::Both => (64, 64),
        }
    }

    pub fn bg_large_tiles(&self, bg: BG) -> bool {
        match bg {
            BG::_1 => self.bg_mode.contains(BGMode::BG1_TILE_SIZE),
            BG::_2 => self.bg_mode.contains(BGMode::BG2_TILE_SIZE),
            BG::_3 => self.bg_mode.contains(BGMode::BG3_TILE_SIZE),
            BG::_4 => self.bg_mode.contains(BGMode::BG4_TILE_SIZE),
        }
    }

    pub fn get_bg_scroll_x(&self, bg: BG) -> usize {
        (match bg {
            BG::_1 => self.bg1_scroll_x & BG_SCROLL_MASK,
            BG::_2 => self.bg2_scroll_x & BG_SCROLL_MASK,
            BG::_3 => self.bg3_scroll_x & BG_SCROLL_MASK,
            BG::_4 => self.bg4_scroll_x & BG_SCROLL_MASK,
        }) as usize
    }

    pub fn get_bg_scroll_y(&self, bg: BG) -> usize {
        (match bg {
            BG::_1 => self.bg1_scroll_y & BG_SCROLL_MASK,
            BG::_2 => self.bg2_scroll_y & BG_SCROLL_MASK,
            BG::_3 => self.bg3_scroll_y & BG_SCROLL_MASK,
            BG::_4 => self.bg4_scroll_y & BG_SCROLL_MASK,
        }) as usize
    }

    pub fn bg_mosaic_enabled(&self, bg: BG) -> bool {
        let empty_mask = (self.mosaic_settings & Mosaic::PIXEL_SIZE).is_empty();
        match bg {
            BG::_1 => self.mosaic_settings.contains(Mosaic::BG1_ENABLE) && !empty_mask,
            BG::_2 => self.mosaic_settings.contains(Mosaic::BG2_ENABLE) && !empty_mask,
            BG::_3 => self.mosaic_settings.contains(Mosaic::BG3_ENABLE) && !empty_mask,
            BG::_4 => self.mosaic_settings.contains(Mosaic::BG4_ENABLE) && !empty_mask,
        }
    }

    pub fn bg_mosaic_mask(&self) -> u8 {
        ((self.mosaic_settings & Mosaic::PIXEL_SIZE).bits() >> 4)
    }

    // Takes a screen coordinate as input (0-255, 0-224).
    // Provides a background pixel location as output.
    // Note that the output pixel might fall outside the range [0-1023].
    pub fn calc_mode_7(&self, x: i16, y: i16) -> (u16, u16) {
        let x_0 = I8F8::from_bits(self.mode7_centre_x as i16);
        let y_0 = I8F8::from_bits(self.mode7_centre_y as i16);
        let x_i = I8F8::from_bits(x) + I8F8::from_bits(self.get_mode7_scroll_x() as i16) - x_0;
        let y_i = I8F8::from_bits(y) + I8F8::from_bits(self.get_mode7_scroll_y() as i16) - y_0;
        let x_out = (I8F8::from_bits(self.mode7_matrix_a as i16) * x_i) + (I8F8::from_bits(self.mode7_matrix_b as i16) * y_i) + x_0;
        let y_out = (I8F8::from_bits(self.mode7_matrix_c as i16) * x_i) + (I8F8::from_bits(self.mode7_matrix_d as i16) * y_i) + y_0;

        (x_out.to_bits() as u16, y_out.to_bits() as u16)
    }

    pub fn get_mode7_scroll_x(&self) -> i16 {
        self.mode7_scroll_x as i16  // TODO: sign extend 13th bit and ignore top 3 (same for centre_x)
    }

    pub fn get_mode7_scroll_y(&self) -> i16 {
        self.mode7_scroll_y as i16  // TODO: sign extend 13th bit and ignore top 3 (same for centre_y)
    }

    // Returns the extend setting.
    pub fn mode_7_extend(&self) -> Mode7Extend {
        Mode7Extend::from(self.mode7_settings)
    }

    pub fn mode_7_flip_x(&self) -> bool {
        self.mode7_settings.contains(Mode7Settings::FLIP_X)
    }

    pub fn mode_7_flip_y(&self) -> bool {
        self.mode7_settings.contains(Mode7Settings::FLIP_Y)
    }
}

// More complex methods called from renderer.
/*impl Registers {
    // Get height of pattern table from start address, in tiles.
    pub fn get_pattern_table_height(&self, pattern_addr: u16, bits_per_pixel: u32) -> u32 {
        let borders = self.get_vram_borders();  // TODO: call this from outside.

        // Find border after pattern addr.
        let end_addr = if let Some(idx) = borders.iter().position(|a| *a == pattern_addr) {
            if (idx + 1) < borders.len() {
                borders[idx + 1] as u32
            } else {
                VRAM_END_ADDR
            }
        } else {
            VRAM_END_ADDR
        };

        let height = (end_addr - pattern_addr as u32) / (16 * 8 * bits_per_pixel);

        if height < PATTERN_MAX_HEIGHT {
            height
        } else {
            PATTERN_MAX_HEIGHT
        }
    }

    // Get starting locations for each block of memory, in order.
    pub fn get_vram_borders(&self) -> Vec<u16> {
        let mode = self.get_mode();
        let mut borders = BTreeSet::new();

        // Always push sprite pattern mem
        borders.insert(self.obj0_pattern_addr());
        borders.insert(self.objn_pattern_addr());

        self.set_bg_borders(&mut borders, self.bg_map_addr(0), self.bg1_settings);
        borders.insert(self.bg_pattern_addr(0));

        self.set_bg_borders(&mut borders, self.bg_map_addr(1), self.bg2_settings);
        borders.insert(self.bg_pattern_addr(1));

        if (mode == 0) || (mode == 1) {
            self.set_bg_borders(&mut borders, self.bg_map_addr(2), self.bg3_settings);
            borders.insert(self.bg_pattern_addr(2));
        }
        if mode == 0 {
            self.set_bg_borders(&mut borders, self.bg_map_addr(3), self.bg4_settings);
            borders.insert(self.bg_pattern_addr(3));
        }

        borders.iter().cloned().collect::<Vec<_>>()
    }

    // Set borders for a background.
    fn set_bg_borders(&self, borders: &mut BTreeSet<u16>, start_addr: u16, settings: BGReg) {
        use MapMirror::*;
        let bg_map_mirror = MapMirror::from(settings);

        borders.insert(start_addr);

        match bg_map_mirror {
            None =>     {},
            X | Y =>    {
                borders.insert(start_addr + SUB_MAP_SIZE);
            },
            Both => {
                borders.insert(start_addr + SUB_MAP_SIZE);
                borders.insert(start_addr + (SUB_MAP_SIZE * 2));
                borders.insert(start_addr + (SUB_MAP_SIZE * 3));
            },
        }
    }
}*/

impl Registers {
    // Get ranges of VRAM in which patterns exist.
    pub fn get_vram_pattern_regions(&self) -> Vec<(u16, u16)> {
        const ROW_HEIGHT_2BPP: u16 = 16 * 16;
        const ROW_HEIGHT_4BPP: u16 = 16 * 32;

        let mode = self.get_mode();
        let mut regions = Vec::new();

        let obj0_pattern_start = self.obj0_pattern_addr();
        let obj0_pattern_end = std::cmp::max(obj0_pattern_start - 1 + (16 * ROW_HEIGHT_4BPP), std::u16::MAX);
        regions.push((obj0_pattern_start, obj0_pattern_end));
        let objn_pattern_start = self.objn_pattern_addr();
        let objn_pattern_end = std::cmp::max(objn_pattern_start - 1 + (16 * ROW_HEIGHT_4BPP), std::u16::MAX);
        regions.push((objn_pattern_start, objn_pattern_end));

        let bg1_pattern_start = self.bg_pattern_addr(BG::_1);
        let bg1_pattern_end = match mode {
            0 => std::cmp::max(bg1_pattern_start - 1 + (64 * ROW_HEIGHT_2BPP), std::u16::MAX),
            1 | 2 | 5 | 6 => std::cmp::max(bg1_pattern_start - 1 + (64 * ROW_HEIGHT_4BPP), std::u16::MAX),
            3 | 4 => std::u16::MAX,
            _ => bg1_pattern_start,
        };
        regions.push((bg1_pattern_start, bg1_pattern_end));

        if mode < 6 {
            let bg2_pattern_start = self.bg_pattern_addr(BG::_2);
            let bg2_pattern_end = match mode {
                0 | 4 | 5 => std::cmp::max(bg2_pattern_start - 1 + (64 * ROW_HEIGHT_2BPP), std::u16::MAX),
                _ => std::cmp::max(bg2_pattern_start - 1 + (64 * ROW_HEIGHT_4BPP), std::u16::MAX),
            };
            regions.push((bg2_pattern_start, bg2_pattern_end));
        }

        if mode < 2 {
            let bg3_pattern_start = self.bg_pattern_addr(BG::_3);
            let bg3_pattern_end = std::cmp::max(bg3_pattern_start - 1 + (64 * ROW_HEIGHT_2BPP), std::u16::MAX);
            regions.push((bg3_pattern_start, bg3_pattern_end));
        }

        if mode == 0 {
            let bg4_pattern_start = self.bg_pattern_addr(BG::_4);
            let bg4_pattern_end = std::cmp::max(bg4_pattern_start - 1 + (64 * ROW_HEIGHT_2BPP), std::u16::MAX);
            regions.push((bg4_pattern_start, bg4_pattern_end));
        }

        regions
    }

    pub fn get_pattern_table_height(&self, bg: BG) -> u16 {
        const TILE_SIZE_2BPP: u16 = 16;
        const TILE_SIZE_4BPP: u16 = 32;
        const TILE_SIZE_8BPP: u16 = 64;

        let max_space = 0x10000 - (self.bg_pattern_addr(bg) as u32);
        let rows = (max_space / 16) as u16;
        match bg {
            BG::_1 => match self.get_mode() {
                0 => std::cmp::min(64, rows / TILE_SIZE_2BPP),
                1 | 2 | 5 | 6 => std::cmp::min(64, rows / TILE_SIZE_4BPP),
                3 | 4 => std::cmp::min(64, rows / TILE_SIZE_8BPP),
                _ => 0
            },
            BG::_2 => match self.get_mode() {
                0 | 4 | 5 => std::cmp::min(64, rows / TILE_SIZE_2BPP),
                _ => std::cmp::min(64, rows / TILE_SIZE_4BPP),
            },
            BG::_3 => std::cmp::min(64, rows / TILE_SIZE_2BPP),
            BG::_4 => std::cmp::min(64, rows / TILE_SIZE_2BPP),
        }
    }
}