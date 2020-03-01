// Other video registers, for BG settings

use bitflags::bitflags;

use std::collections::BTreeSet;

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
    pub struct Mosaic: u8 {
        const PIXEL_SIZE = bits![7, 6, 5, 4];
        const BG4_ENABLE = bit!(3);
        const BG3_ENABLE = bit!(2);
        const BG2_ENABLE = bit!(1);
        const BG1_ENABLE = bit!(0);
    }
}

const VRAM_END_ADDR: u32 = 64 * 1024;
const PATTERN_MAX_HEIGHT: u32 = 64;
const BG_SCROLL_MASK: u16 = 0x3FF;

// Sub-map size (32x32 tiles)
const SUB_MAP_LEN: u16 = 32;
const SUB_MAP_SIZE: u16 = SUB_MAP_LEN * SUB_MAP_LEN * 2;

pub struct Registers {

        screen_display:     u8,
        object_settings:    ObjectSettings,
        bg_mode:            BGMode,
        mosaic_settings:    Mosaic,

        bg1_settings:       BGReg,
        bg2_settings:       BGReg,
        bg3_settings:       BGReg,
        bg4_settings:       BGReg,

    pub bg12_char_addr:     u8,
    pub bg34_char_addr:     u8,

        bg1_scroll_x:       u16,
        bg1_scroll_y:       u16,
        bg2_scroll_x:       u16,
        bg2_scroll_y:       u16,
        bg3_scroll_x:       u16,
        bg3_scroll_y:       u16,
        bg4_scroll_x:       u16,
        bg4_scroll_y:       u16,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            screen_display:     0,
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
        }
    }

    // Setters (CPU side)
    pub fn set_screen_display(&mut self, data: u8) {
        self.screen_display = data;
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
    }

    pub fn set_bg1_scroll_y(&mut self, data: u8) {
        self.bg1_scroll_y = make16!(data, hi!(self.bg1_scroll_y));
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

    // Getters for the renderer.
    pub fn get_screen_display(&self) -> u8 {
        self.screen_display
    }

    pub fn get_mode(&self) -> u8 {
        (self.bg_mode & BGMode::MODE).bits()
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

    pub fn get_bg_settings(&self, bg: usize) -> BGReg {
        match bg {
            0 => self.bg1_settings,
            1 => self.bg2_settings,
            2 => self.bg3_settings,
            3 => self.bg4_settings,
            _ => unreachable!()
        }
    }

    // TODO: use less magic numbers in the following.
    pub fn bg_pattern_addr(&self, bg: usize) -> u16 {
        match bg {
            0 => ((self.bg12_char_addr & 0xF) as u16) << 13,
            1 => ((self.bg12_char_addr & 0xF0) as u16) << 9,
            2 => ((self.bg34_char_addr & 0xF) as u16) << 13,
            3 => ((self.bg34_char_addr & 0xF0) as u16) << 9,
            _ => unreachable!()
        }
    }

    pub fn bg_map_addr(&self, bg: usize) -> u16 {
        match bg {
            0 => ((self.bg1_settings & BGReg::ADDR).bits() as u16) << 9,
            1 => ((self.bg2_settings & BGReg::ADDR).bits() as u16) << 9,
            2 => ((self.bg3_settings & BGReg::ADDR).bits() as u16) << 9,
            3 => ((self.bg4_settings & BGReg::ADDR).bits() as u16) << 9,
            _ => unreachable!()
        }
    }

    pub fn bg_large_tiles(&self, bg: usize) -> bool {
        match bg {
            0 => self.bg_mode.contains(BGMode::BG1_TILE_SIZE),
            1 => self.bg_mode.contains(BGMode::BG2_TILE_SIZE),
            2 => self.bg_mode.contains(BGMode::BG3_TILE_SIZE),
            3 => self.bg_mode.contains(BGMode::BG4_TILE_SIZE),
            _ => unreachable!()
        }
    }

    pub fn get_bg_scroll_x(&self, bg: usize) -> u16 {
        match bg {
            0 => self.bg1_scroll_x & BG_SCROLL_MASK,
            1 => self.bg2_scroll_x & BG_SCROLL_MASK,
            2 => self.bg3_scroll_x & BG_SCROLL_MASK,
            3 => self.bg4_scroll_x & BG_SCROLL_MASK,
            _ => unreachable!()
        }
    }

    pub fn get_bg_scroll_y(&self, bg: usize) -> u16 {
        match bg {
            0 => self.bg1_scroll_y & BG_SCROLL_MASK,
            1 => self.bg2_scroll_y & BG_SCROLL_MASK,
            2 => self.bg3_scroll_y & BG_SCROLL_MASK,
            3 => self.bg4_scroll_y & BG_SCROLL_MASK,
            _ => unreachable!()
        }
    }

    pub fn bg_mosaic_enabled(&self, bg: usize) -> bool {
        let empty_mask = (self.mosaic_settings & Mosaic::PIXEL_SIZE).is_empty();
        match bg {
            0 => self.mosaic_settings.contains(Mosaic::BG1_ENABLE) && !empty_mask,
            1 => self.mosaic_settings.contains(Mosaic::BG2_ENABLE) && !empty_mask,
            2 => self.mosaic_settings.contains(Mosaic::BG3_ENABLE) && !empty_mask,
            3 => self.mosaic_settings.contains(Mosaic::BG4_ENABLE) && !empty_mask,
            _ => unreachable!()
        }
    }

    pub fn bg_mosaic_mask(&self) -> u8 {
        ((self.mosaic_settings & Mosaic::PIXEL_SIZE).bits() >> 4) + 1
    }

    // Other checks
    pub fn in_fblank(&self) -> bool {
        test_bit!(self.screen_display, 7, u8)
    }
}

// More complex methods called from renderer.
impl Registers {
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
}