// Sprite mem.
use vulkano::{
    buffer::CpuBufferPool,
    device::Device
};

use super::super::{
    TexSide, Vertex, VertexBuffer,
    super::ram::ObjectSettings
};

use std::sync::Arc;

const Y_FLIP_BIT: u32 = 15;
const X_FLIP_BIT: u32 = 14;

const LINE_HEIGHT: f32 = 1.0 / 112.0;

const SMALL: u32 = 0;
const LARGE: u32 = bit!(23, u32);

const ATLAS_SIZE: f32 = 16.0 * 8.0; // Atlas width/height (16x16 tiles of 8x8 pixels)

pub struct SpriteMem {
    small_size:     (u8, u8),   // Size of sprites in pixels.
    large_size:     (u8, u8),

    small_tex_size: [f32; 2],   // Size of sprite textures relative to atlas.
    large_tex_size: [f32; 2],

    settings:       u8,

    buffer:         Vec<Vertex>,
    buffer_pool:    CpuBufferPool<Vertex>
}

impl SpriteMem {
    pub fn new(device: &Arc<Device>) -> Self {
        SpriteMem {
            small_size:     (8, 8),
            large_size:     (16, 16),

            small_tex_size: [8.0 / ATLAS_SIZE, 8.0 / ATLAS_SIZE],
            large_tex_size: [16.0 / ATLAS_SIZE, 16.0 / ATLAS_SIZE],

            settings:       0,

            buffer:         Vec::new(),
            buffer_pool:    CpuBufferPool::vertex_buffer(device.clone())
        }
    }

    // Return true if settings are the same. Otherwise, setup new sprite sizes and return false.
    pub fn check_and_set_obj_settings(&mut self, settings: u8) -> bool {
        if settings != self.settings {
            let (small, large) = match (ObjectSettings::from_bits_truncate(settings) & ObjectSettings::SIZE).bits() >> 5 {
                0 => ((8, 8), (16, 16)),
                1 => ((8, 8), (32, 32)),
                2 => ((8, 8), (64, 64)),
                3 => ((16, 16), (32, 32)),
                4 => ((16, 16), (64, 64)),
                5 => ((32, 32), (64, 64)),
                6 => ((16, 32), (32, 64)),
                7 => ((16, 32), (32, 32)),
                _ => unreachable!()
            };

            self.small_size = small;
            self.large_size = large;

            self.small_tex_size = [(small.0 as f32) / ATLAS_SIZE, (small.1 as f32) / ATLAS_SIZE];
            self.large_tex_size = [(large.0 as f32) / ATLAS_SIZE, (large.1 as f32) / ATLAS_SIZE];

            self.settings = settings;

            false
        } else {
            true
        }
    }

    pub fn get_vertex_buffer_0(&mut self, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        self.make_vertex_buffer(0, y, oam_hi, oam_lo)
    }

    pub fn get_vertex_buffer_n(&mut self, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        self.make_vertex_buffer(1, y, oam_hi, oam_lo)
    }

    pub fn get_small_tex_size(&self) -> [f32; 2] {
        self.small_tex_size
    }

    pub fn get_large_tex_size(&self) -> [f32; 2] {
        self.large_tex_size
    }
}

// Internal
impl SpriteMem {
    fn make_vertex_buffer(&mut self, name_table_select: u8, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        self.buffer.clear();

        for lo in (0..oam_lo.len()).step_by(4) {
            let name_table = oam_lo[lo + 3] & 1;
            if name_table == name_table_select {    // TODO: check the name table in GPU
                let hi_addr = lo / 16;
                let shift_amt = ((lo / 4) % 4) * 2;
                let hi = (oam_hi[hi_addr] >> shift_amt) & bits![1, 0];
                self.make_vertices(y, &oam_lo[lo..(lo + 4)], hi)
            }
        }

        if self.buffer.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(self.buffer.drain(..)).unwrap())
        }
    }

    // Make vertices for a sprite on a line.
    fn make_vertices(&mut self, line_y: u8, oam_lo: &[u8], hi: u8) {
        let large = test_bit!(hi, 1, u8);
        let size = if large {self.large_size} else {self.small_size};
        let y_pos = oam_lo[1];
        let bottom_y = y_pos.wrapping_add(size.1 - 1);

        // See if this sprite appear on this line.
        let should_render = if bottom_y > y_pos {
            (line_y >= y_pos) && (line_y <= bottom_y)
        } else {
            (line_y >= y_pos) || (line_y <= bottom_y)
        };

        if should_render {
            let x_pos = make16!(if test_bit!(hi, 0, u8) {0xFF} else {0}, oam_lo[0]) as i16;
            let x_left = ((x_pos as f32) / 128.0) - 1.0;
            let x_right = x_left + (size.0 as f32 / 128.0); // TODO: check for wraparound

            let y_top = ((line_y as f32) / 112.0) - 1.0;
            let y_bottom = y_top + LINE_HEIGHT;

            let tile_data = make16!(oam_lo[3], oam_lo[2]) as u32;

            let (left, right) = if test_bit!(tile_data, X_FLIP_BIT, u32) {
                (TexSide::Right, TexSide::Left)
            } else {
                (TexSide::Left, TexSide::Right)
            };

            let base_tex_y = line_y.wrapping_sub(y_pos);
            let tex_y = (if test_bit!(tile_data, Y_FLIP_BIT, u32) {
                (size.1 - 1) - base_tex_y
            } else {
                base_tex_y
            } as u32) << 17;

            let size = if large {LARGE} else {SMALL};

            self.buffer.push(Vertex{ position: [x_left, y_top],     data: size | tex_y | left as u32 | tile_data });
            self.buffer.push(Vertex{ position: [x_right, y_top],    data: size | tex_y | right as u32 | tile_data });
            self.buffer.push(Vertex{ position: [x_left, y_bottom],  data: size | tex_y | left as u32 | tile_data });
            self.buffer.push(Vertex{ position: [x_right, y_top],    data: size | tex_y | right as u32 | tile_data });
            self.buffer.push(Vertex{ position: [x_left, y_bottom],  data: size | tex_y | left as u32 | tile_data });
            self.buffer.push(Vertex{ position: [x_right, y_bottom], data: size | tex_y | right as u32 | tile_data });
        }
    }
}