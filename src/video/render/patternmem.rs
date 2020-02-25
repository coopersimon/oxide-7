// Pattern mem for a single background. Reads VRAM, outputs Texture data.

#[derive(Clone, Copy, PartialEq)]
pub enum BitsPerPixel {
    _2 = 2,
    _4 = 4,
    _8 = 8
}

#[derive(Clone)]
pub struct Tile {
    data:   Vec<Vec<u8>> // Raw data. One byte for each pixel.
}

impl Tile {
    pub fn new() -> Self {
        Tile {
            data: vec![vec![0; 8]; 8]
        }
    }

    #[inline]
    pub fn get_texel(&self, x: usize, y: usize) -> u8 {
        self.data[y][x]
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

    // Set the address.
    pub fn set_addr(&mut self, start_addr: u16, width: usize, height: usize) {
        let size = (width / 8) * (height / 8) * match self.bits_per_pixel {
            BitsPerPixel::_2 => 16,
            BitsPerPixel::_4 => 32,
            BitsPerPixel::_8 => 64,
        } - 1;

        self.start_addr = start_addr;
        self.end_addr = start_addr + (size as u16);

        self.tiles.clear();
    }

    // Return the BPP.
    pub fn get_bits_per_pixel(&self) -> BitsPerPixel {
        self.bits_per_pixel
    }

    // Return the start address.
    pub fn get_start_addr(&self) -> u16 {
        self.start_addr
    }

    // Return the end address.
    pub fn get_end_addr(&self) -> u16 {
        self.end_addr
    }

    // Make the tiles. Input raw data, width and height in TILES, and bits per pixel.
    // Rows are always 16 tiles.
    pub fn make_tiles(&mut self, data: &[u8], width: usize, height: usize) {
        self.tiles.resize_with(width * height, || Tile::new());

        match self.bits_per_pixel {
            // 16 bytes per tile.
            BitsPerPixel::_2 => self.make_tiles_2bpp(data),
            // 32 bytes per tile.
            BitsPerPixel::_4 => self.make_tiles_4bpp(data),
            // 64 bytes per tile.
            BitsPerPixel::_8 => self.make_tiles_8bpp(data),
        }
    }

    // Ref a tile.
    pub fn ref_tile<'a>(&'a self, num: usize) -> &'a Tile {
        &self.tiles[num]
    }
}

// Internal
impl PatternMem {
    /*fn make_image_2bpp(&mut self, data: &[u8]) {
        let row_size = 16 * 8;      // Row length in pixels.
        let mut col = 0;            // Current col of tile in pixels.
        let mut row = 0;            // Current row of tile in pixels.
        let mut y = 0;              // Current Y coord in pixels. (To the nearest tile.)

        for (i, d) in data.iter().enumerate() {
            let bitplane = i % 2;
            let base_index = ((y + row) * row_size) + col;

            for x in 0..8 {
                let bit = (*d >> (7 - x)) & 1;
                self.tex_data[base_index + x] |= bit << bitplane;
            }

            if bitplane != 0 {
                // Next row in tile.
                row += 1;

                // Next tile.
                if row >= 8 {
                    row = 0;

                    col += 8;
                    if col >= row_size {
                        col = 0;
                        y += 8;
                    }
                }
            }
        }
    }*/

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

    /*fn make_image_4bpp(&mut self, data: &[u8]) {
        let row_size = 16 * 8;      // Row length in pixels.
        let mut col = 0;            // Current col of tile in pixels.
        let mut row = 0;            // Current row of tile in pixels.
        let mut y = 0;              // Current Y coord in pixels. (To the nearest tile.)

        let mut bitplane_offset = 0;    // As bitplanes are stored in pairs.

        for (i, d) in data.iter().enumerate() {
            let sub_bitplane = i % 2;
            let bitplane = sub_bitplane + bitplane_offset;
            let base_index = ((y + row) * row_size) + col;

            for x in 0..8 {
                let bit = (*d >> (7 - x)) & 1;
                self.tex_data[base_index + x] |= bit << bitplane;
            }

            if sub_bitplane != 0 {
                // Next row in tile.
                row += 1;
                if row >= 8 {
                    row = 0;

                    bitplane_offset += 2;
                    // Next tile.
                    if bitplane_offset >= 4 {
                        bitplane_offset = 0;

                        col += 8;
                        if col >= row_size {
                            col = 0;
                            y += 8;
                        }
                    }
                }
            }
        }

    }*/

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

    /*fn make_image_8bpp(&mut self, data: &[u8]) {
        let row_size = 16 * 8;      // Row length in pixels.
        let mut col = 0;            // Current col of tile in pixels.
        let mut row = 0;            // Current row of tile in pixels.
        let mut y = 0;              // Current Y coord in pixels. (To the nearest tile.)

        let mut x = 0;

        for d in data.iter() {
            let base_index = ((y + row) * row_size) + col;

            self.tex_data[base_index + x] = *d;

            x += 1;
            if x >= 8 {
                // Next row in tile.
                row += 1;
                if row >= 8 {
                    row = 0;

                    // Next tile.
                    col += 8;
                    if col >= row_size {
                        col = 0;
                        y += 8;
                    }
                }
            }
        }

    }*/

    fn make_tiles_8bpp(&mut self, data: &[u8]) {
        for (i, d) in data.iter().enumerate() {
            let x = i % 8;
            let y = (i / 8) % 8;
            let tile_num = i / 64;

            self.tiles[tile_num].data[y][x] = *d;
        }
    }
}