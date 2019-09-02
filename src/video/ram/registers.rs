// Other video registers, for BG settings

use bitflags::bitflags;

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
        bg_mode:            BGMode,
        mosaic_settings:    Mosaic,

    pub bg1_settings:       u8,
    pub bg2_settings:       u8,
    pub bg3_settings:       u8,
    pub bg4_settings:       u8,

    pub bg12_char_addr:     u8,
    pub bg34_char_addr:     u8,

    pub bg1_scroll_x:       u8,
    pub bg1_scroll_y:       u8,
    pub bg2_scroll_x:       u8,
    pub bg2_scroll_y:       u8,
    pub bg3_scroll_x:       u8,
    pub bg3_scroll_y:       u8,
    pub bg4_scroll_x:       u8,
    pub bg4_scroll_y:       u8,
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            screen_display:     0,
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

    pub fn set_bg_mode(&mut self, data: u8) {
        self.bg_mode = BGMode::from_bits_truncate(data);
    }

    pub fn set_mosaic(&mut self, data: u8) {
        self.mosaic_settings = Mosaic::from_bits_truncate(data);
    }

    // Getters for the renderer.
    pub fn get_mode(&self) -> u8 {
        (self.bg_mode & BGMode::MODE).bits()
    }

    pub fn bg_1_pattern_addr(&self) -> u16 {
        ((self.bg12_char_addr & 0xF) as u16) << 12
    }

    pub fn bg_2_pattern_addr(&self) -> u16 {
        ((self.bg12_char_addr & 0xF0) as u16) << 8
    }

    pub fn bg_3_pattern_addr(&self) -> u16 {
        ((self.bg34_char_addr & 0xF) as u16) << 12
    }

    pub fn bg_4_pattern_addr(&self) -> u16 {
        ((self.bg34_char_addr & 0xF0) as u16) << 8
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