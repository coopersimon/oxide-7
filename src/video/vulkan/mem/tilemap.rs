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

// Sub-map size (32x32 tiles)
const SUB_MAP_LEN: usize = 32;
const SUB_MAP_SIZE: usize = SUB_MAP_LEN * SUB_MAP_LEN * 2;

// Tile sizes TODO: alter based on mode
const SMALL_TILE: usize = 8;
const LARGE_TILE: usize = 16;

// BG Register bits.
bitflags! {
    #[derive(Default)]
    struct BGReg: u8 {
        const ADDR      = bit!(7) | bit!(6) | bit!(5) | bit!(4) | bit!(3) | bit!(2);
        const MIRROR_Y  = bit!(1);
        const MIRROR_X  = bit!(0);
    }
}

// Combination of mirror X and Y.
enum MapMirror {
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

pub struct TileMap {
    vertices:       Vec<Vertex>,

    bg_reg:         BGReg,      // Address and size as stored in register.

    start_addr:     u16,        // Start address of tile map.
    //size:           u16,        // Size of tile map in bytes.
    row_len:        usize,      // Length of a row of vertices.

    large_tiles:    bool,       // If true, tiles are 16 pixels high/wide (depending on mode).
    pixel_height:   u16,        // Height of map in pixels.
    map_size:       (f32, f32), // Size of map relative to viewport.

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

        let view_size = (if large_tiles {16} else {32}, if large_tiles {14} else {28});
        let tile_height = if large_tiles {LARGE_TILE} else {SMALL_TILE};

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

        let start_addr = ((reg_bits & BGReg::ADDR).bits() as u16) << 9;

        TileMap {
            vertices:       vertices,

            bg_reg:         reg_bits,

            start_addr:     start_addr,
            //size:           (grid_size_x * grid_size_y * 2) as u16,
            row_len:        grid_size_x * 6,

            large_tiles:    large_tiles,
            pixel_height:   (grid_size_y * tile_height) as u16,
            map_size:       ((grid_size_x as f32 / view_size.0 as f32) * 2.0, (grid_size_y as f32 / view_size.1 as f32) * 2.0),

            buffer_pool:    CpuBufferPool::vertex_buffer(device.clone())
        }
    }

    // Check if size is the same. If it is, set the new start address. If not, return false.
    pub fn check_and_set_addr(&mut self, settings: u8, large_tiles: bool) -> bool {
        let new_bg_reg = BGReg::from_bits_truncate(settings);
        if (new_bg_reg != self.bg_reg) || (large_tiles != self.large_tiles) {
            if ((new_bg_reg & (BGReg::MIRROR_X | BGReg::MIRROR_Y)) != (self.bg_reg & (BGReg::MIRROR_X | BGReg::MIRROR_Y))) || (large_tiles != self.large_tiles) {
                return false;   // This map needs to be recreated.
            } else {
                self.start_addr = ((new_bg_reg & BGReg::ADDR).bits() as u16) << 9;
                self.bg_reg = new_bg_reg;
                // TODO: Force update
            }
        }
        
        true
    }

    // Update the tiles if the memory region is dirty.
    pub fn update(&mut self, mem: &VideoMem) {
        use MapMirror::*;
        // First A:
        if mem.vram_dirty_range(self.start_addr, self.start_addr + (SUB_MAP_SIZE - 1) as u16) {
            self.create_submap_vertex_data(mem, 0, 0, 0);
        }
        match self.map_mirror() {
            None => {},
            X => {
                // B
                if mem.vram_dirty_range(self.start_addr + SUB_MAP_SIZE as u16, self.start_addr + ((SUB_MAP_SIZE * 2) - 1) as u16) {
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE, SUB_MAP_LEN, 0);
                }
            },
            Y => {
                // B
                if mem.vram_dirty_range(self.start_addr + SUB_MAP_SIZE as u16, self.start_addr + ((SUB_MAP_SIZE * 2) - 1) as u16) {
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE, 0, SUB_MAP_LEN);
                }
            },
            Both => {
                if mem.vram_dirty_range(self.start_addr + SUB_MAP_SIZE as u16, self.start_addr + ((SUB_MAP_SIZE * 2) - 1) as u16) {
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE, SUB_MAP_LEN, 0);  // B
                }
                if mem.vram_dirty_range(self.start_addr + (SUB_MAP_SIZE * 2) as u16, self.start_addr + ((SUB_MAP_SIZE * 3) - 1) as u16) {
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE * 2, 0, SUB_MAP_LEN);  // C
                }
                if mem.vram_dirty_range(self.start_addr + (SUB_MAP_SIZE * 3) as u16, self.start_addr + ((SUB_MAP_SIZE * 4) - 1) as u16) {
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE * 3, SUB_MAP_LEN, SUB_MAP_LEN);    // D
                }
            }
        }
    }

    // Get a line of vertices without priority bit set.
    pub fn get_lo_vertex_buffer(&mut self, y: u16) -> Option<VertexBuffer> {
        let start = self.row_len * y as usize;
        let tile_map = self.vertices.iter()
                .skip(start)
                .take(self.row_len)
                .filter(|v| !test_bit!(v.data, PRIORITY_BIT, u32))
                .cloned()
                .collect::<Vec<_>>();

        if tile_map.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(tile_map).unwrap())
        }
    }

    // Get a line of vertices with priority bit set.
    pub fn get_hi_vertex_buffer(&mut self, y: u16) -> Option<VertexBuffer> {
        let start = self.row_len * y as usize;
        let tile_map = self.vertices.iter()
                .skip(start)
                .take(self.row_len)
                .filter(|v| test_bit!(v.data, PRIORITY_BIT, u32))
                .cloned()
                .collect::<Vec<_>>();

        if tile_map.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(tile_map).unwrap())
        }
    }

    /*pub fn get_tile_width(&self) -> f32 {
        
    }*/

    pub fn get_tile_height(&self) -> usize {
        if self.large_tiles {LARGE_TILE} else {SMALL_TILE}
    }

    // Tile map size, relative to viewport size.
    pub fn get_map_size(&self) -> (f32, f32) {
        self.map_size
    }

    // Tile map height in pixels.
    pub fn get_pixel_height(&self) -> u16 {
        self.pixel_height
    }
}

// Internal
impl TileMap {
    // Store data for a single sub-map in the vertices.
    // Pass in the x and y offsets in tiles (0 or 32).
    fn create_submap_vertex_data(&mut self, mem: &VideoMem, start_offset: usize, x_offset: usize, y_offset: usize) {
        let start_addr = (self.start_addr as usize) + start_offset;
        let tile_height = self.get_tile_height();
        let mut lo = 0;

        for (i, data) in mem.get_vram().iter().skip(start_addr).take(SUB_MAP_SIZE).enumerate() {
            if (i % 2) == 0 {
                lo = *data;
            } else {
                let x_tile = ((i / 2) % SUB_MAP_LEN) + x_offset;
                let y_tile = ((i / 2) / SUB_MAP_LEN) + y_offset;

                let tile_data = make16!(*data, lo) as u32;

                let (left, right) = if test_bit!(tile_data, X_FLIP_BIT, u32) {
                    (Side::Right, Side::Left)
                } else {
                    (Side::Left, Side::Right)
                };

                let y_coords = if test_bit!(tile_data, Y_FLIP_BIT, u32) {
                    (0..(tile_height as u32)).rev().collect::<Vec<u32>>()
                } else {
                    (0..(tile_height as u32)).collect::<Vec<u32>>()
                };

                let index = (y_tile * self.row_len * tile_height) + (x_tile * 6);

                for (j, y) in (index..(index + (self.row_len * tile_height))).step_by(self.row_len).zip(&y_coords) {
                    let line_y = *y << 17;
                    self.vertices[j].data =     tile_data | line_y | left as u32;
                    self.vertices[j + 1].data = tile_data | line_y | left as u32;
                    self.vertices[j + 2].data = tile_data | line_y | right as u32;
                    self.vertices[j + 3].data = tile_data | line_y | left as u32;
                    self.vertices[j + 4].data = tile_data | line_y | right as u32;
                    self.vertices[j + 5].data = tile_data | line_y | right as u32;
                }
            }
        }
    }

    // Tile map mirror.
    fn map_mirror(&self) -> MapMirror {
        MapMirror::from(self.bg_reg)
    }
}