// Tile maps for backgrounds.
// XxY tiles in size, where X and Y can be 32 and 64 respectively.
// Each tile can be 8x8 or 16x16.

use vulkano::{
    buffer::{
        BufferUsage,
        CpuBufferPool,
        ImmutableBuffer
    },
    device::{
        Device,
        Queue
    },
    sync::{
        now,
        GpuFuture
    }
};

use bitflags::bitflags;

use std::sync::Arc;

use super::super::{
    Vertex,
    VertexBuffer,
    VertexSide,
    TexSide,
    super::VideoMem,
    super::ram::{
        MapMirror,
        BGReg
    }
};

// Tile data bits (that we care about here).
const Y_FLIP_BIT: u32   = 15;
const X_FLIP_BIT: u32   = 14;

// Sub-map size (32x32 tiles)
const SUB_MAP_LEN: usize = 32;
const SUB_MAP_SIZE: usize = SUB_MAP_LEN * SUB_MAP_LEN * 2;

// Tile sizes TODO: alter based on mode
const SMALL_TILE: usize = 8;
const LARGE_TILE: usize = 16;

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
    current_buffer: Option<VertexBuffer>,
    index_buffers:  Vec<Arc<ImmutableBuffer<[u32]>>>,  // An index buffer for each line.
}

impl TileMap {
    // Grid and view size in tiles.
    // Grid size is 32 or 64.
    // View size is (32 | 16, 28 | 14)
    // Tile size in pixels.
    pub fn new(device: &Arc<Device>, queue: &Arc<Queue>, bg_reg: BGReg, large_tiles: bool) -> Self {
        let grid_size_x = if bg_reg.contains(BGReg::MIRROR_X) {64} else {32};
        let grid_size_y = if bg_reg.contains(BGReg::MIRROR_Y) {64} else {32};

        let view_size = (if large_tiles {16} else {32}, if large_tiles {14} else {28});
        let tile_height = if large_tiles {LARGE_TILE} else {SMALL_TILE};
        let row_len = grid_size_x * 6;

        let mut vertices = Vec::new();
        let mut index_buffers = Vec::new();
        let mut index_futures = Vec::new(); // TODO: handle these a bit better.

        let x_frac = 2.0 / view_size.0 as f32;
        let y_frac = (2.0 / view_size.1 as f32) / (tile_height as f32);  // Each y tile is 8 or 16 lines high.
        let mut lo_y = -1.0;
        let mut hi_y = lo_y + y_frac;

        for y in 0..(grid_size_y * tile_height) {
            let mut indices = Vec::new();

            let y_coord = ((y % tile_height) << 17) as u32;
            let mut left_x = -1.0;
            let mut right_x = left_x + x_frac;
            for x in 0..grid_size_x {
                vertices.push(Vertex{ position: [left_x, lo_y], data: y_coord | VertexSide::Left as u32 });
                vertices.push(Vertex{ position: [left_x, hi_y], data: y_coord | VertexSide::Left as u32 });
                vertices.push(Vertex{ position: [right_x, lo_y], data: y_coord | VertexSide::Right as u32 });
                vertices.push(Vertex{ position: [left_x, hi_y], data: y_coord | VertexSide::Left as u32 });
                vertices.push(Vertex{ position: [right_x, lo_y], data: y_coord | VertexSide::Right as u32 });
                vertices.push(Vertex{ position: [right_x, hi_y], data: y_coord | VertexSide::Right as u32 });

                left_x = right_x;
                right_x += x_frac;

                let base = ((y * row_len) + (x * 6)) as u32;
                indices.push(base);
                indices.push(base + 1);
                indices.push(base + 2);
                indices.push(base + 3);
                indices.push(base + 4);
                indices.push(base + 5);
            }

            lo_y = hi_y;
            hi_y += y_frac;

            let (index_buffer, index_future) = ImmutableBuffer::from_iter(indices.into_iter(), BufferUsage::index_buffer(), queue.clone()).unwrap();
            index_buffers.push(index_buffer);
            index_futures.push(Box::new(index_future) as Box<dyn GpuFuture>);
        }

        let init_future = Box::new(now(device.clone())) as Box<dyn GpuFuture>;
        index_futures.drain(..).fold(init_future, |all, f| Box::new(all.join(f)) as Box<dyn GpuFuture>).flush().unwrap();

        let start_addr = ((bg_reg & BGReg::ADDR).bits() as u16) << 9;

        println!("Making new");

        TileMap {
            vertices:       vertices,

            bg_reg:         bg_reg,

            start_addr:     start_addr,
            //size:           (grid_size_x * grid_size_y * 2) as u16,
            row_len:        row_len,

            large_tiles:    large_tiles,
            pixel_height:   (grid_size_y * tile_height) as u16,
            map_size:       ((grid_size_x as f32 / view_size.0 as f32) * 2.0, (grid_size_y as f32 / view_size.1 as f32) * 2.0),

            buffer_pool:    CpuBufferPool::vertex_buffer(device.clone()),
            current_buffer: None,
            index_buffers:  index_buffers
        }
    }

    // Check if size is the same. If it is, set the new start address. If not, return false.
    pub fn check_and_set_addr(&mut self, settings: BGReg, large_tiles: bool) -> bool {
        /*if (new_bg_reg != self.bg_reg) || (large_tiles != self.large_tiles) {
            if ((new_bg_reg & (BGReg::MIRROR_X | BGReg::MIRROR_Y)) != (self.bg_reg & (BGReg::MIRROR_X | BGReg::MIRROR_Y))) || (large_tiles != self.large_tiles) {
                return false;   // This map needs to be recreated.
            } else {
                self.start_addr = ((new_bg_reg & BGReg::ADDR).bits() as u16) << 9;
                self.bg_reg = new_bg_reg;
                // TODO: Force update
            }
        }*/
        
        !((settings != self.bg_reg) || (large_tiles != self.large_tiles))
    }

    // Update the tiles if the memory region is dirty.
    pub fn update(&mut self, mem: &VideoMem) {
        use MapMirror::*;
        let mut changed = false;
        // First A:
        if mem.vram_is_dirty(self.start_addr) {
            changed = true;
            self.create_submap_vertex_data(mem, 0, 0, 0);
        }
        match self.map_mirror() {
            None => {},
            X => {
                // B
                if mem.vram_is_dirty(self.start_addr + SUB_MAP_SIZE as u16) {
                    changed = true;
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE, SUB_MAP_LEN, 0);
                }
            },
            Y => {
                // B
                if mem.vram_is_dirty(self.start_addr + SUB_MAP_SIZE as u16) {
                    changed = true;
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE, 0, SUB_MAP_LEN);
                }
            },
            Both => {
                if mem.vram_is_dirty(self.start_addr + SUB_MAP_SIZE as u16) {
                    changed = true;
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE, SUB_MAP_LEN, 0);  // B
                }
                if mem.vram_is_dirty(self.start_addr + (SUB_MAP_SIZE * 2) as u16) {
                    changed = true;
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE * 2, 0, SUB_MAP_LEN);  // C
                }
                if mem.vram_is_dirty(self.start_addr + (SUB_MAP_SIZE * 3) as u16) {
                    changed = true;
                    self.create_submap_vertex_data(mem, SUB_MAP_SIZE * 3, SUB_MAP_LEN, SUB_MAP_LEN);    // D
                }
            }
        }

        if changed || self.current_buffer.is_none() {
            self.current_buffer = Some(self.buffer_pool.chunk(
                self.vertices.iter().cloned()
            ).unwrap());
        }
    }

    pub fn get_vertex_buffer(&self) -> VertexBuffer {
        self.current_buffer.as_ref().unwrap().clone()
    }

    pub fn get_index_buffer(&self, line: u16) -> Arc<ImmutableBuffer<[u32]>> {
        self.index_buffers[line as usize].clone()
    }

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
                    (TexSide::Right as u32 | VertexSide::Left as u32, TexSide::Left as u32 | VertexSide::Right as u32)
                } else {
                    (TexSide::Left as u32 | VertexSide::Left as u32, TexSide::Right as u32 | VertexSide::Right as u32)
                };

                let y_coords = if test_bit!(tile_data, Y_FLIP_BIT, u32) {
                    (0..(tile_height as u32)).rev().collect::<Vec<u32>>()
                } else {
                    (0..(tile_height as u32)).collect::<Vec<u32>>()
                };

                let index = (y_tile * self.row_len * tile_height) + (x_tile * 6);

                for (j, y) in (index..(index + (self.row_len * tile_height))).step_by(self.row_len).zip(&y_coords) {
                    let line_y = *y << 17;
                    self.vertices[j].data =     tile_data | line_y | left;
                    self.vertices[j + 1].data = tile_data | line_y | left;
                    self.vertices[j + 2].data = tile_data | line_y | right;
                    self.vertices[j + 3].data = tile_data | line_y | left;
                    self.vertices[j + 4].data = tile_data | line_y | right;
                    self.vertices[j + 5].data = tile_data | line_y | right;
                }
            }
        }
    }

    // Tile map mirror.
    fn map_mirror(&self) -> MapMirror {
        MapMirror::from(self.bg_reg)
    }
}