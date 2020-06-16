// Cache for holding bitmap pixel values.
use bitflags::bitflags;

use super::super::ScreenMode;

bitflags! {
    #[derive(Default)]
    struct PlotOption: u8 {
        const OBJ_MODE = bit!(4);
        const FREEZE_HI = bit!(3);
        const HI_NYBBLE = bit!(2);
        const DITHER = bit!(1);
        const TRANSPARENT = bit!(0);
    }
}

#[derive(PartialEq, Clone, Copy)]
enum BPP {
    _2,
    _4,
    _8,
}

impl From<ScreenMode> for BPP {
    fn from(val: ScreenMode) -> Self {
        match (val & ScreenMode::MD).bits() {
            0 => BPP::_2,
            1 => BPP::_4,
            _ => BPP::_8
        }
    }
}

enum ScreenHeight {
    _128,
    _160,
    _192,
    Obj
}

impl From<ScreenMode> for ScreenHeight {
    fn from(val: ScreenMode) -> Self {
        if val.contains(ScreenMode::HT0 | ScreenMode::HT1) {
            ScreenHeight::Obj
        } else if val.contains(ScreenMode::HT0) {
            ScreenHeight::_160
        } else if val.contains(ScreenMode::HT1) {
            ScreenHeight::_192
        } else {
            ScreenHeight::_128
        }
    }
}

// Represents a row of 8 pixels.
// Each pixel is 2, 4, or 8 bits.
#[derive(Clone)]
struct PrimaryCache {
    data:   [u8; 8],
    bitp:   u8, // Bit-pending flags
    tile_x: u8,
    y:      u8,
}

impl PrimaryCache {
    fn new() -> Self {
        Self {
            data:   [0; 8],
            bitp:   0,
            tile_x: 0,
            y:      0,
        }
    }

    fn init(&mut self, x: u8, y: u8) {
        for d in self.data.iter_mut() {
            *d = 0;
        }
        self.bitp = 0;
        self.tile_x = x;
        self.y = y;
    }

    // Write a pixel to the cache from an x address.
    fn write_pix(&mut self, x: u8, data: u8) {
        self.data[x as usize] = data;
        self.bitp |= bit!(x);
    }

    // Flush the cache.
    // Packs the pixel bits into bitplanes.
    // The buffer size in bytes should equal the bits per pixel.
    // Each bitplane appears in pairs, 16 bytes apart in memory.
    /*fn flush(&mut self, buffer: &mut [[u8; 2]]) {
        for (bitplane_pair, sub_buffer) in buffer.iter_mut().enumerate() {
            let bitplane_base = bitplane_pair * 2;
            for (bitplane_offset, out) in sub_buffer.iter_mut().enumerate() {
                let bitplane = bitplane_base + bitplane_offset;
                *out = self.data.iter().enumerate().fold(0, |acc, (i, data)| {
                    let bit = (data >> bitplane) & 1;
                    let shift_amt = 7 - i;
                    acc | (bit << shift_amt)
                });
            }
        }
        self.bitp = 0;
    }*/

    fn is_dirty(&self) -> bool {
        self.bitp != 0
    }
}

// Used with rpix (when loading from memory)
impl From<&SecondaryCache> for PrimaryCache {
    fn from(s: &SecondaryCache) -> Self {
        let mut primary = PrimaryCache::new();
        primary.tile_x = s.tile_x;
        primary.y = s.y;
        for (bitplane, d) in s.data.iter().enumerate() {
            for (x, cache_data) in primary.data.iter_mut().enumerate() {
                let shift_amt = 7 - x;
                let bit = (*d >> shift_amt) & 1;
                *cache_data |= bit << bitplane;
            }
        }
        //println!("Ready to write {},{} : {:?} -> {:?}", s.tile_x, s.y, s.data, primary.data);
        primary
    }
}

// Data as it is stored in memory.
// 2, 4, or 8 bitplanes of 8 bits.
#[derive(Clone)]
struct SecondaryCache {
    data:   [u8; 8],
    tile_x: u8,
    y:      u8,
}

impl SecondaryCache {
    fn new() -> Self {
        Self {
            data:   [0; 8],
            tile_x: 0,
            y:      0,
        }
    }

    fn get_bitplanes(&mut self, bpp: BPP) -> Vec<u8> {
        let size = match bpp {
            BPP::_2 => 2,
            BPP::_4 => 4,
            BPP::_8 => 8,
        };
        self.data.iter().cloned().take(size).collect()
    }

    // Fill the buffer.
    fn fill(&mut self, data: &[u8]) {
        for (i, out) in self.data.iter_mut().enumerate() {
            *out = if i < data.len() {
                data[i]
            } else {
                0
            };
        }
    }
}

impl From<&PrimaryCache> for SecondaryCache {
    fn from(p: &PrimaryCache) -> Self {
        let mut secondary = SecondaryCache::new();
        secondary.tile_x = p.tile_x;
        secondary.y = p.y;
        for (bitplane, out) in secondary.data.iter_mut().enumerate() {
            *out = p.data.iter().enumerate().fold(0, |acc, (x, data)| {
                let bit = (data >> bitplane) & 1;
                let shift_amt = 7 - x;
                acc | (bit << shift_amt)
            });
        }
        //println!("Ready to flush {},{} : {:?} -> {:?}", p.tile_x, p.y, p.data, secondary.data);
        secondary
    }
}

pub struct PixelCache {
    primary:    PrimaryCache,
    secondary:  SecondaryCache,

    screen_base:    u32,
    screen_mode:    ScreenMode,
    colr:           u8,
    por:            PlotOption,

    bpp:            BPP,
    height:         ScreenHeight,
}

impl PixelCache {
    pub fn new() -> Self {
        Self {
            primary:    PrimaryCache::new(),
            secondary:  SecondaryCache::new(),

            screen_base:    0,
            screen_mode:    ScreenMode::default(),
            colr:           0,
            por:            PlotOption::default(),

            bpp:            BPP::_2,
            height:         ScreenHeight::_128,
        }
    }

    pub fn set_screen_base(&mut self, data: u8) {
        self.screen_base = 0x70_0000 + (0x400 * (data as u32));
    }

    pub fn set_screen_mode(&mut self, mode: ScreenMode) {
        self.screen_mode = mode;
        self.bpp = mode.into();
        self.height = if self.por.contains(PlotOption::OBJ_MODE) {
            ScreenHeight::Obj
        } else {
            mode.into()
        };
    }

    pub fn set_colr(&mut self, data: u8) {
        let to_write = if self.por.contains(PlotOption::HI_NYBBLE) {
            let hi = hi_nybble!(data);
            (hi << 4) | hi
        } else {
            data
        };
        self.colr = if self.por.contains(PlotOption::FREEZE_HI) {
            (self.colr & 0xF0) | (to_write & 0xF)
        } else {
            to_write
        };
    }

    pub fn set_por(&mut self, data: u8) {
        self.por = PlotOption::from_bits_truncate(data);
        self.height = if self.por.contains(PlotOption::OBJ_MODE) {
            ScreenHeight::Obj
        } else {
            self.screen_mode.into()
        };
    }

    // Plot the pixel.
    // If false is returned, the pixel could not be written.
    pub fn try_plot(&mut self, x: u8, y: u8) -> bool {
        let tile_x = x / 8;
        if self.primary.y == y && self.primary.tile_x == tile_x {
            self.do_plot(x % 8, y);
            true
        } else {
            false
        }
    }

    // Get the tile address for the specified coord.
    pub fn calc_tile_addr(&mut self, x: u8, y: u8) -> u32 {
        let tile_x = (x / 8) as u32;
        let tile_y = (y / 8) as u32;
        let tile_num = match self.height {
            ScreenHeight::_128 => (tile_x * 0x10) + tile_y,
            ScreenHeight::_160 => (tile_x * 0x14) + tile_y,
            ScreenHeight::_192 => (tile_x * 0x18) + tile_y,
            ScreenHeight::Obj => {
                let hi_x = (tile_x / 0x10) * 0x100;
                let hi_y = (tile_y / 0x10) * 0x200;
                let lo_x = tile_x % 0x10;
                let lo_y = (tile_y % 0x10) * 0x10;
                lo_x + lo_y + hi_x + hi_y
            },
        };

        let y_idx = ((y % 8) * 2) as u32;
        
        self.screen_base + match self.bpp {
            BPP::_2 => (tile_num * 0x10),
            BPP::_4 => (tile_num * 0x20),
            BPP::_8 => (tile_num * 0x40),
        } + y_idx
    }
}

// Flushing
impl PixelCache {
    // Flushes data from the secondary cache.
    // Returns the data and address to write to.
    pub fn flush(&mut self) -> Option<(Vec<u8>, u32)> {
        if self.primary.is_dirty() {
            self.primary.bitp = 0;
            self.secondary = (&self.primary).into();
            let addr = self.calc_tile_addr(self.secondary.tile_x * 8, self.secondary.y);
            let data = self.secondary.get_bitplanes(self.bpp);
            Some((data, addr))
        } else {
            None
        }
    }

    pub fn bpp(&self) -> usize {
        match self.bpp {
            BPP::_2 => 2,
            BPP::_4 => 4,
            BPP::_8 => 8,
        }
    }
}

// RPIX
impl PixelCache {

    // Returns an address to fill to if the secondary cache needs it.
    pub fn get_fill_addr(&mut self, x: u8, y: u8) -> Option<u32> {
        if self.secondary.tile_x == (x / 8) && self.secondary.y == y {
            None    // Already filled.
        } else {
            // Setup secondary to be written into.
            self.secondary.tile_x = x / 8;
            self.secondary.y = y;
            Some(self.calc_tile_addr(x, y))
        }
    }

    // Fills the secondary cache line.
    // Assumes it has already been flushed!
    pub fn fill(&mut self, data: &[u8]) {
        /*self.primary.init(x / 8, y);
        for (i, d) in buffer.iter().enumerate() {
            for (x, cache_data) in self.primary.data.iter_mut().enumerate() {
                let shift_amt = 7 - x;
                let bit = (*d >> shift_amt) & 1;
                *cache_data |= bit << i;
            }
        }*/
        self.secondary.fill(data);
        self.primary = (&self.secondary).into();
    }

    // Reads pixel from the primary cache.
    pub fn read_pixel(&self, x: u8) -> u8 {
        self.primary.data[x as usize]
    }
}

// internal
impl PixelCache {
    fn do_plot(&mut self, x: u8, y: u8) {
        let colour = if self.por.contains(PlotOption::DITHER) && self.bpp != BPP::_8 {
            if test_bit!(x ^ y, 0, u8) {
                hi_nybble!(self.colr)
            } else {
                lo_nybble!(self.colr)
            }
        } else {
            self.colr
        };
        let masked_colour = match self.bpp {
            BPP::_2 => colour & 0x3,
            BPP::_4 => colour & 0xF,
            BPP::_8 => colour,
        };
        if self.should_plot(masked_colour) {
            self.primary.write_pix(x, masked_colour);
        }
    }

    fn should_plot(&self, col: u8) -> bool {
        let mask = if self.por.contains(PlotOption::FREEZE_HI) {0xF} else {0xFF};
        self.por.contains(PlotOption::TRANSPARENT) || (col & mask) != 0
    }
}