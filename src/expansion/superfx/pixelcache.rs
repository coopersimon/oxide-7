// Cache for holding bitmap pixel values.
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct ScreenMode: u8 {
        const HT1 = bit!(5);
        const RON = bit!(4);
        const RAN = bit!(3);
        const HT0 = bit!(2);
        const MD = bits![1, 0];
    }
}

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
struct CacheLine {
    data:   [u8; 8],
    bitp:   u8, // Bit-pending flags
    tile_x: u8,
    y:      u8,
}

impl CacheLine {
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
    fn flush(&mut self, buffer: &mut [[u8; 2]]) {
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
    }

    fn is_dirty(&self) -> bool {
        self.bitp != 0
    }
}

pub struct PixelCache {
    primary:    CacheLine,
    secondary:  CacheLine,

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
            primary:    CacheLine::new(),
            secondary:  CacheLine::new(),

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

    // Returns true if RON is set.
    pub fn set_screen_mode(&mut self, data: u8) -> bool {
        self.screen_mode = ScreenMode::from_bits_truncate(data);
        self.bpp = self.screen_mode.into();
        self.height = if self.por.contains(PlotOption::OBJ_MODE) {
            ScreenHeight::Obj
        } else {
            self.screen_mode.into()
        };
        //println!("Screen mode: {:X}", data);
        self.screen_mode.contains(ScreenMode::RON)
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
        //println!("Plot option: {:X}", data);
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
    // While this is true, keep calling flush.
    pub fn needs_flush(&self) -> bool {
        self.secondary.is_dirty()
    }

    // Flushes a single cache line.
    // Also returns base address to flush to.
    pub fn flush(&mut self, buffer: &mut [[u8; 2]]) -> u32 {
        if self.secondary.is_dirty() {
            self.secondary.flush(buffer);
            self.calc_tile_addr(self.secondary.tile_x * 8, self.secondary.y)
        } else {
            0
        }
    }

    // Number of pairs of buffers needed to flush pixel cache.
    pub fn flush_bitplane_pairs(&self) -> usize {
        match self.bpp {
            BPP::_2 => 1,
            BPP::_4 => 2,
            BPP::_8 => 4,
        }
    }

    pub fn bpp(&self) -> u32 {
        match self.bpp {
            BPP::_2 => 2,
            BPP::_4 => 4,
            BPP::_8 => 8,
        }
    }
}

// RPIX
impl PixelCache {

    // Fills the primary cache line.
    // Assumes it has already been flushed!
    pub fn fill(&mut self, x: u8, y: u8, buffer: &[u8]) {
        self.transfer_to_secondary();
        self.primary.init(x / 8, y);
        for (i, d) in buffer.iter().enumerate() {
            for (x, cache_data) in self.primary.data.iter_mut().enumerate() {
                let shift_amt = 7 - x;
                let bit = (*d >> shift_amt) & 1;
                *cache_data |= bit << i;
            }
        }
    }

    // Reads pixel from the primary cache.
    pub fn read_pixel(&self, x: u8) -> u8 {
        self.primary.data[x as usize]
    }
}

// internal
impl PixelCache {
    fn transfer_to_secondary(&mut self) {
        self.secondary = self.primary.clone();
        self.primary.bitp = 0;
    }

    fn do_plot(&mut self, x: u8, y: u8) {
        let colour = if self.por.contains(PlotOption::DITHER) {
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
        /*let masked_colour = match self.height {
            ScreenHeight::_128 => 0,
            ScreenHeight::_160 => 3,
            ScreenHeight::_192 => 0,
            ScreenHeight::Obj => 5
        };*/
        if self.should_plot(masked_colour) {
            self.primary.write_pix(x, masked_colour);
        }
    }

    fn should_plot(&self, col: u8) -> bool {
        let mask = if self.por.contains(PlotOption::FREEZE_HI) {0xF} else {0xFF};
        self.por.contains(PlotOption::TRANSPARENT) || (col & mask) != 0
    }
}