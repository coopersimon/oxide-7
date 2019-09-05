// Tile maps for backgrounds.
// XxY tiles in size, where X and Y can be 32 and 64 respectively.
// Each tile can be 8x8 or 16x16.

use vulkano::{
    buffer::CpuBufferPool,
    device::Device,
};

use bitflags::bitflags;

use std::sync::Arc;

use super::super::{
    Vertex,
    VertexBuffer,
    Side,
    super::VideoMem
};

// Tile data bits (that we care about here).
const Y_FLIP_BIT: u32   = 15;
const X_FLIP_BIT: u32   = 14;
const PRIORITY_BIT: u32 = 13;

// BG Register bits.
bitflags! {
    #[derive(Default)]
    struct BGReg: u8 {
        const ADDR      = bit!(7) | bit!(6) | bit!(5) | bit!(4) | bit!(3) | bit!(2);
        const MIRROR_Y  = bit!(1);
        const MIRROR_X  = bit!(0);
    }
}

pub struct TileMap {
    vertices:       Vec<Vertex>,

    bg_reg:         BGReg, // Address and size as stored in register.

    start_addr:     u16,
    size:           u16,
    row_len:        usize,
    tile_height:    usize,

    buffer_pool:    CpuBufferPool<Vertex>,
}

impl TileMap {
    // Grid and view size in tiles.
    // Grid size is 32 or 64.
    // View size is (32 | 16, 28 | 14)
    // Tile size in pixels.
    pub fn new(device: &Arc<Device>, bg_reg: u8, large_tiles: bool) -> Self {
        let reg_bits = BGReg::from_bits_truncate(bg_reg);
        let grid_size_x = if reg_bits.contains(BGReg::MIRROR_X) {64} else {32};
        let grid_size_y = if reg_bits.contains(BGReg::MIRROR_Y) {64} else {32};

        let view_size = (if large_tiles {16} else {32}, if large_tiles {16} else {32});
        let tile_height = if large_tiles {16} else {8};

        let mut vertices = Vec::new();

        let x_frac = 2.0 / view_size.0 as f32;
        let y_frac = (2.0 / view_size.1 as f32) / (tile_height as f32);  // Each y tile is 8 or 16 lines high.
        let mut lo_y = -1.0;
        let mut hi_y = lo_y + y_frac;

        for y in 0..(grid_size_y * tile_height) {
            let y_coord = ((y % tile_height) << 17) as u32;
            let mut left_x = -1.0;
            let mut right_x = left_x + x_frac;
            for _ in 0..grid_size_x {
                vertices.push(Vertex{ position: [left_x, lo_y], data: y_coord | Side::Left as u32 });
                vertices.push(Vertex{ position: [left_x, hi_y], data: y_coord | Side::Left as u32 });
                vertices.push(Vertex{ position: [right_x, lo_y], data: y_coord | Side::Right as u32 });
                vertices.push(Vertex{ position: [left_x, hi_y], data: y_coord | Side::Left as u32 });
                vertices.push(Vertex{ position: [right_x, lo_y], data: y_coord | Side::Right as u32 });
                vertices.push(Vertex{ position: [right_x, hi_y], data: y_coord | Side::Right as u32 });

                left_x = right_x;
                right_x += x_frac;
            }
            lo_y = hi_y;
            hi_y += y_frac;
        }

        let start_addr = ((reg_bits & BGReg::ADDR).bits() as u16) << 8;

        TileMap {
            vertices:       vertices,

            bg_reg:         reg_bits,

            start_addr:     start_addr,
            size:           (grid_size_x * grid_size_y * 2) as u16,
            row_len:        grid_size_x * 6,
            tile_height:    tile_height,

            buffer_pool:    CpuBufferPool::vertex_buffer(device.clone())
        }
    }

    // Check if size is the same. If it is, set the new start address. If not, return false.
    pub fn check_and_set_addr(&mut self, settings: u8) -> bool {
        let new_bg_reg = BGReg::from_bits_truncate(settings);
        if new_bg_reg != self.bg_reg {
            if (new_bg_reg & (BGReg::MIRROR_X | BGReg::MIRROR_Y)) != (self.bg_reg & (BGReg::MIRROR_X | BGReg::MIRROR_Y)) {
                return false;   // This map needs to be recreated.
            } else {
                self.start_addr = ((new_bg_reg & BGReg::ADDR).bits() as u16) << 8;
                self.bg_reg = new_bg_reg;
            }
        }
        
        true
    }

    // Update the tiles if the memory region is dirty.
    pub fn update(&mut self, mem: &VideoMem) {
        if mem.vram_dirty_range(self.start_addr, self.start_addr + self.size) {
            let mut lo = 0;
            for (i, data) in mem.get_vram().iter().skip(self.start_addr as usize).take(self.size as usize).enumerate() {
                if (i % 2) == 0 {
                    lo = *data;
                } else {
                    let y_offset = ((i / 2) / self.row_len) * self.tile_height;
                    let index = y_offset + ((i / 2) % self.row_len) * 6;

                    let tile_data = make16!(*data, lo) as u32;

                    let (left, right) = if test_bit!(tile_data, X_FLIP_BIT, u32) {
                        (Side::Right, Side::Left)
                    } else {
                        (Side::Left, Side::Right)
                    };

                    let y_coords = if test_bit!(tile_data, Y_FLIP_BIT, u32) {
                        (0..(self.tile_height as u32)).rev().collect::<Vec<u32>>()
                    } else {
                        (0..(self.tile_height as u32)).collect::<Vec<u32>>()
                    };

                    for (j, y) in (index..(index + (self.row_len * self.tile_height))).step_by(self.row_len).zip(&y_coords) {
                        let y = y << 17;
                        self.vertices[j].data =     tile_data | y | left as u32;
                        self.vertices[j + 1].data = tile_data | y | left as u32;
                        self.vertices[j + 2].data = tile_data | y | right as u32;
                        self.vertices[j + 3].data = tile_data | y | left as u32;
                        self.vertices[j + 4].data = tile_data | y | right as u32;
                        self.vertices[j + 5].data = tile_data | y | right as u32;
                    }
                }
            }
        }
    }

    // Get a line of vertices without priority bit set.
    pub fn get_lo_vertex_buffer(&mut self, y: u8) -> Option<VertexBuffer> {
        let start = self.row_len * y as usize;
        let tile_map = self.vertices.iter()
                .skip(start)
                .take(self.row_len)
                .cloned()
                .filter(|v| !test_bit!(v.data, PRIORITY_BIT, u32))
                .collect::<Vec<_>>();

        if tile_map.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(tile_map).unwrap())
        }
    }

    // Get a line of vertices with priority bit set.
    pub fn get_hi_vertex_buffer(&mut self, y: u8) -> Option<VertexBuffer> {
        let start = self.row_len * y as usize;
        let tile_map = self.vertices.iter()
                .skip(start)
                .take(self.row_len)
                .cloned()
                .filter(|v| test_bit!(v.data, PRIORITY_BIT, u32))
                .collect::<Vec<_>>();

        if tile_map.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(tile_map).unwrap())
        }
    }

    /*pub fn get_tile_width(&self) -> f32 {
        
    }*/

    pub fn get_tile_height(&self) -> f32 {
        self.tile_height as f32
    }

    // Tile map width in tiles
    pub fn width(&self) -> u32 {
        if self.bg_reg.contains(BGReg::MIRROR_X) {64} else {32}
    }

    // Tile map height in tiles
    pub fn height(&self) -> u32 {
        if self.bg_reg.contains(BGReg::MIRROR_Y) {64} else {32}
    }
}