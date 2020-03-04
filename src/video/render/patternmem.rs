// Pattern mem for a single background. Reads VRAM, outputs Texture data.

pub const TILE_SIZE: usize = 8;
const PATTERN_MEM_WIDTH_TILES: u16 = 16;
const OBJ_PATTERN_MEM_HEIGHT_TILES: u16 = 16;

#[derive(Clone, Copy, PartialEq)]
pub enum BitsPerPixel {
    _2 = 2,
    _4 = 4,
    _8 = 8
}

#[derive(Clone)]
pub struct Tile {
    pub data:   Vec<Vec<u8>> // Raw data. One byte for each pixel.
}

impl Tile {
    pub fn new() -> Self {
        Tile {
            data: vec![vec![0; TILE_SIZE]; TILE_SIZE]
        }
    }

    #[inline]
    pub fn get_texel(&self, x: usize, y: usize) -> u8 {
        self.data[y][x]
    }

    #[inline]
    pub fn clear(&mut self) {
        for row in self.data.iter_mut() {
            for tex in row.iter_mut() {
                *tex = 0;
            }
        }
    }
}

pub struct PatternMem {
    // Parameters
    bits_per_pixel: BitsPerPixel,

    start_addr:     u16,
    end_addr:       u16,

    tiles:          Vec<Tile>
}

impl PatternMem {
    pub fn new(bits_per_pixel: BitsPerPixel) -> Self {
        PatternMem {
            bits_per_pixel: bits_per_pixel,

            start_addr:     std::u16::MAX,
            end_addr:       std::u16::MAX,

            tiles:          Vec::new()
        }
    }

    // Set the address, plus height in TILES.
    pub fn set_addr(&mut self, start_addr: u16, height: u16) {
        let num_tiles = PATTERN_MEM_WIDTH_TILES * height;
        let size = num_tiles * match self.bits_per_pixel {
            BitsPerPixel::_2 => 16,
            BitsPerPixel::_4 => 32,
            BitsPerPixel::_8 => 64,
        } - 1;

        self.start_addr = start_addr;
        self.end_addr = start_addr + size;

        self.tiles.resize_with(num_tiles as usize, || Tile::new());
    }

    // Set the address for sprite memory. (Always 4BPP, height=16)
    pub fn set_addr_obj(&mut self, start_addr: u16) {
        let num_tiles = PATTERN_MEM_WIDTH_TILES * OBJ_PATTERN_MEM_HEIGHT_TILES;
        let size = (num_tiles * 32) - 1;

        self.start_addr = start_addr;
        self.end_addr = start_addr + size;

        self.tiles.resize_with(num_tiles as usize, || Tile::new());
    }

    // Return the BPP.
    pub fn get_bits_per_pixel(&self) -> BitsPerPixel {
        self.bits_per_pixel
    }

    // Return the start address.
    pub fn get_start_addr(&self) -> u16 {
        self.start_addr
    }

    // Make the tiles.
    pub fn make_tiles(&mut self, data: &[u8]) {
        let start = self.start_addr as usize;
        let end = self.end_addr as usize;

        for tile in self.tiles.iter_mut() {
            tile.clear();
        }

        match self.bits_per_pixel {
            // 16 bytes per tile.
            BitsPerPixel::_2 => self.make_tiles_2bpp(&data[start..=end]),
            // 32 bytes per tile.
            BitsPerPixel::_4 => self.make_tiles_4bpp(&data[start..=end]),
            // 64 bytes per tile.
            BitsPerPixel::_8 => self.make_tiles_8bpp(&data[start..=end]),
        }
    }

    // Ref a tile.
    pub fn ref_tile<'a>(&'a self, num: usize) -> &'a Tile {
        &self.tiles[num % self.tiles.len()] // TODO: replace this with bitwise &
    }
}

// Internal
impl PatternMem {
    fn make_tiles_2bpp(&mut self, data: &[u8]) {
        for (i, d) in data.iter().enumerate() {
            let y = (i / 2) % 8;
            let bitplane = i % 2;
            let tile_num = i / 16;
            
            for x in 0..8 {
                let bit = (*d >> (7 - x)) & 1;
                self.tiles[tile_num].data[y][x] |= bit << bitplane;
            }
        }
    }

    fn make_tiles_4bpp(&mut self, data: &[u8]) {
        for (i, d) in data.iter().enumerate() {
            let y = (i / 2) % 8;

            let sub_bitplane = i % 2;
            let bitplane_offset = (i / 16) % 2;
            let bitplane = sub_bitplane + (bitplane_offset << 1);

            let tile_num = i / 32;
            
            for x in 0..8 {
                let bit = (*d >> (7 - x)) & 1;
                self.tiles[tile_num].data[y][x] |= bit << bitplane;
            }
        }
    }

    fn make_tiles_8bpp(&mut self, data: &[u8]) {
        for (i, d) in data.iter().enumerate() {
            let y = (i / 2) % 8;

            let sub_bitplane_0 = i % 2;
            let sub_bitplane_1 = (i / 16) % 2;
            let bitplane_offset = (i / 32) % 2;
            let bitplane = sub_bitplane_0 + (sub_bitplane_1 << 1) + (bitplane_offset << 2);

            let tile_num = i / 64;
            
            for x in 0..8 {
                let bit = (*d >> (7 - x)) & 1;
                self.tiles[tile_num].data[y][x] |= bit << bitplane;
            }
        }
    }
}
