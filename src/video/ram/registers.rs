// Other video registers, for BG settings

use bitflags::bitflags;

use std::collections::BTreeSet;

const VRAM_END_ADDR: u32 = 64 * 1024;
const PATTERN_MAX_HEIGHT: u32 = 64;

bitflags! {
    #[derive(Default)]
    pub struct ObjectSettings: u8 {
        const SIZE = bit!(7) | bit!(6) | bit!(5);
        const SELECT = bit!(4) | bit!(3);
        const BASE = bit!(2) | bit!(1) | bit!(0);
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
        const MODE          = bit!(2) | bit!(1) | bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Mosaic: u8 {
        const PIXEL_SIZE = bit!(7) | bit!(6) | bit!(5) | bit!(4);
        const BG4_ENABLE = bit!(3);
        const BG3_ENABLE = bit!(2);
        const BG2_ENABLE = bit!(1);
        const BG1_ENABLE = bit!(0);
    }
}

pub struct Registers {

        screen_display:     u8,
        object_settings:    ObjectSettings,
        bg_mode:            BGMode,
        mosaic_settings:    Mosaic,

    pub bg1_settings:       u8,
    pub bg2_settings:       u8,
    pub bg3_settings:       u8,
    pub bg4_settings:       u8,

    pub bg12_char_addr:     u8,
    pub bg34_char_addr:     u8,

    pub bg1_scroll_x:       u16,
    pub bg1_scroll_y:       u16,
    pub bg2_scroll_x:       u16,
    pub bg2_scroll_y:       u16,
    pub bg3_scroll_x:       u16,
    pub bg3_scroll_y:       u16,
    pub bg4_scroll_x:       u16,
    pub bg4_scroll_y:       u16,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            screen_display:     0,
            object_settings:    ObjectSettings::default(),
            bg_mode:            BGMode::default(),
            mosaic_settings:    Mosaic::default(),

            bg1_settings:       0,
            bg2_settings:       0,
            bg3_settings:       0,
            bg4_settings:       0,

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

    pub fn obj_0_pattern_addr(&self) -> u16 {
        let base = (self.object_settings & ObjectSettings::BASE).bits() as u16;
        base << 13
    }

    pub fn obj_n_pattern_addr(&self) -> u16 {
        let base = (self.object_settings & ObjectSettings::BASE).bits() as u16;
        let table = (self.object_settings & ObjectSettings::SELECT).bits() as u16 + 1;
        (base << 13) + (table << 12)
    }

    // TODO: use less magic numbers in the following.
    pub fn bg_1_pattern_addr(&self) -> u16 {
        ((self.bg12_char_addr & 0xF) as u16) << 13
    }

    pub fn bg_2_pattern_addr(&self) -> u16 {
        ((self.bg12_char_addr & 0xF0) as u16) << 9
    }

    pub fn bg_3_pattern_addr(&self) -> u16 {
        ((self.bg34_char_addr & 0xF) as u16) << 13
    }

    pub fn bg_4_pattern_addr(&self) -> u16 {
        ((self.bg34_char_addr & 0xF0) as u16) << 9
    }

    pub fn bg_1_map_addr(&self) -> u16 {
        ((self.bg1_settings & 0xFC) as u16) << 9
    }

    pub fn bg_2_map_addr(&self) -> u16 {
        ((self.bg2_settings & 0xFC) as u16) << 9
    }

    pub fn bg_3_map_addr(&self) -> u16 {
        ((self.bg3_settings & 0xFC) as u16) << 9
    }

    pub fn bg_4_map_addr(&self) -> u16 {
        ((self.bg4_settings & 0xFC) as u16) << 9
    }

    pub fn bg_1_large_tiles(&self) -> bool {
        self.bg_mode.contains(BGMode::BG1_TILE_SIZE)
    }

    pub fn bg_2_large_tiles(&self) -> bool {
        self.bg_mode.contains(BGMode::BG2_TILE_SIZE)
    }

    pub fn bg_3_large_tiles(&self) -> bool {
        self.bg_mode.contains(BGMode::BG3_TILE_SIZE)
    }

    pub fn bg_4_large_tiles(&self) -> bool {
        self.bg_mode.contains(BGMode::BG4_TILE_SIZE)
    }
}

// More complex methods called from renderer.
impl Registers {
    // Get starting locations for each block of memory, in order.
    fn get_vram_borders(&self) -> Vec<u16> {
        let mode = self.get_mode();
        let mut borders = BTreeSet::new();

        // Always push sprite pattern mem
        borders.insert(self.obj_0_pattern_addr());
        borders.insert(self.obj_n_pattern_addr());

        borders.insert(self.bg_1_map_addr());
        borders.insert(self.bg_1_pattern_addr());

        borders.insert(self.bg_2_map_addr());
        borders.insert(self.bg_2_pattern_addr());

        if (mode == 0) || (mode == 1) {
            borders.insert(self.bg_3_map_addr());
            borders.insert(self.bg_3_pattern_addr());
        }
        if mode == 0 {
            borders.insert(self.bg_4_map_addr());
            borders.insert(self.bg_4_pattern_addr());
        }

        borders.iter().cloned().collect::<Vec<_>>()
    }

    // Get height of pattern table from start address, in pixels.
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
            height * 8
        } else {
            PATTERN_MAX_HEIGHT * 8
        }
    }
}