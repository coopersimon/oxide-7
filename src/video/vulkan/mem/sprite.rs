// Sprite mem.
use vulkano::{
    buffer::CpuBufferPool,
    device::Device
};

use bitflags::bitflags;

use super::super::{
    Side, Vertex, VertexBuffer,
    super::ram::ObjectSettings
};

use std::sync::Arc;

// Make tile data for a sprite.
macro_rules! make_tile_data {
    ($hi:expr, $lo:expr) => {{
        let keep = bit!(7) | bit!(6) | bit!(0);
        let palette_bits = bit!(3) | bit!(2) | bit!(1);
        let hi = ($hi & keep) | (($lo & palette_bits) << 1);
        make16!(hi, $lo)
    }};
}

const PRIORITY_BITS: u8 = bit!(5) | bit!(4);
const PRIORITY_0: u8 = 0;
const PRIORITY_1: u8 = 1 << 4;
const PRIORITY_2: u8 = 2 << 4;
const PRIORITY_3: u8 = 3 << 4;

const Y_FLIP_BIT: u32 = 15;
const X_FLIP_BIT: u32 = 14;

const LINE_HEIGHT: f32 = 1.0 / 112.0;

pub struct SpriteMem {
    small_size:     (u8, u8),
    large_size:     (u8, u8),

    buffer_pool:    CpuBufferPool<Vertex>
}

impl SpriteMem {
    pub fn new(device: &Arc<Device>) -> Self {
        SpriteMem {
            small_size:     (8, 8),
            large_size:     (16, 16),
            buffer_pool:    CpuBufferPool::vertex_buffer(device.clone())
        }
    }

    pub fn set_obj_settings(&mut self, settings: u8) {
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
    }

    pub fn get_vertex_buffer_0(&mut self, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        self.get_vertex_buffer(PRIORITY_0, y, oam_hi, oam_lo)
    }

    pub fn get_vertex_buffer_1(&mut self, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        self.get_vertex_buffer(PRIORITY_1, y, oam_hi, oam_lo)
    }

    pub fn get_vertex_buffer_2(&mut self, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        self.get_vertex_buffer(PRIORITY_2, y, oam_hi, oam_lo)
    }

    pub fn get_vertex_buffer_3(&mut self, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        self.get_vertex_buffer(PRIORITY_3, y, oam_hi, oam_lo)
    }
}

// Internal
impl SpriteMem {
    // Check each object's priority and add it to the buffer if we need it.
    fn get_vertex_buffer(&mut self, priority_check: u8, y: u8, oam_hi: &[u8], oam_lo: &[u8]) -> Option<VertexBuffer> {
        let mut buffer = Vec::new();

        for lo in (0..oam_lo.len()).step_by(4) {
            let priority = oam_lo[lo + 3] & PRIORITY_BITS;
            if priority == priority_check {
                let hi_addr = lo / 16;
                let hi = (oam_hi[hi_addr] >> ((lo / 4) % 4)) & 0x3;
                self.make_vertices(y, &oam_lo[lo..=(lo + 3)], hi, &mut buffer)
            }
        }

        if buffer.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(buffer).unwrap())
        }
    }

    // Make vertices for a sprite on a line.
    fn make_vertices(&self, line_y: u8, oam_lo: &[u8], hi: u8, out: &mut Vec<Vertex>) {
        let large = test_bit!(hi, 2, u8);
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
            let x_pos = make16!(if test_bit!(hi, 1, u8) {0xFF} else {0}, oam_lo[0]) as i16;
            let x_left = ((x_pos as f32) / 128.0) - 1.0;
            let x_right = x_left + (size.0 as f32 / 128.0);

            let y_top = ((line_y as f32) / 112.0) - 1.0;
            let y_bottom = y_top + LINE_HEIGHT;

            let tile_data = make_tile_data!(oam_lo[3], oam_lo[2]) as u32;

            let (left, right) = if test_bit!(tile_data, X_FLIP_BIT, u32) {
                (Side::Right, Side::Left)
            } else {
                (Side::Left, Side::Right)
            };

            let base_tex_y = line_y.wrapping_sub(y_pos);
            let tex_y = if test_bit!(tile_data, Y_FLIP_BIT, u32) {
                (size.1 - 1) - base_tex_y
            } else {
                base_tex_y
            } as u32;

            // TODO: communicate tex size somehow.
            out.push(Vertex{ position: [x_left, y_top],     data: tex_y | left as u32 | tile_data });
            out.push(Vertex{ position: [x_right, y_top],    data: tex_y | right as u32 | tile_data });
            out.push(Vertex{ position: [x_left, y_bottom],  data: tex_y | left as u32 | tile_data });
            out.push(Vertex{ position: [x_right, y_top],    data: tex_y | right as u32 | tile_data });
            out.push(Vertex{ position: [x_left, y_bottom],  data: tex_y | left as u32 | tile_data });
            out.push(Vertex{ position: [x_right, y_bottom], data: tex_y | right as u32 | tile_data });
        }
    }
}