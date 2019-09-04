// Converting native VRAM, CGRAM and OAM into Vulkan structures.

mod palette;
pub mod patternmem;
mod sprite;
mod tilemap;

use vulkano::{
    command_buffer::{
        AutoCommandBuffer,
        CommandBufferExecFuture
    },
    device::{
        Device,
        Queue
    },
    image::{
        Dimensions,
        immutable::ImmutableImage
    },
    format::{
        R8Uint
    },
    sync::{
        now, GpuFuture, NowFuture
    }
};

use std::sync::Arc;

use super::VertexBuffer;
use crate::video::VRamRef;
use patternmem::*;
use tilemap::*;

const PATTERN_WIDTH: u32 = 16 * 8; // Pattern width in pixels (16 tiles)
const PATTERN_HEIGHT: u32 = 64 * 8; // Pattern width in pixels (16 tiles)


pub struct MemoryCache {
    native_mem:     VRamRef,
    // Internal settings
    mode:           u8,

    // Internal mem
    pattern_mem:    [PatternMem; 4],
    tile_maps:      [TileMap; 4],

    // Vulkan things
    device:         Arc<Device>,
    queue:          Arc<Queue>
}

impl MemoryCache {
    pub fn new(vram: VRamRef, device: &Arc<Device>, queue: &Arc<Queue>) -> Self {
        let pattern_mem = [
            PatternMem::new(queue, device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0),  // BG 1 can be 2, 4 or 8 BPP
            PatternMem::new(queue, device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0),  // BG 2 can be 2, 4 or 7 BPP
            PatternMem::new(queue, device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0),  // BG 3 can only be 2 BPP
            PatternMem::new(queue, device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0)   // BG 4 can only be 2 BPP
        ];

        let tile_maps = [
            TileMap::new(device, 0, false),
            TileMap::new(device, 0, false),
            TileMap::new(device, 0, false),
            TileMap::new(device, 0, false)
        ];

        MemoryCache {
            native_mem:     vram,

            mode:           8,

            pattern_mem:    pattern_mem,
            tile_maps:      tile_maps,

            device:         device.clone(),
            queue:          queue.clone()
        }
    }

    // Called every line. Checks mode and dirtiness of video memory.
    pub fn init(&mut self) {
        // Check mode and alter backgrounds.
        let stored_mode = self.native_mem.lock().expect("Couldn't lock native mem.").get_registers().get_mode();
        if stored_mode != self.mode {
            self.switch_mode(stored_mode);
        }

        let mut mem = self.native_mem.lock().expect("Couldn't lock native mem.");

        // Check background mem locations
        // TODO: just check relevant BGs
        let regs = mem.get_registers();
        if self.pattern_mem[0].get_start_addr() != regs.bg_1_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg_1_pattern_addr(), self.pattern_mem[0].get_bits_per_pixel() as u32);
            self.pattern_mem[0].set_addr(regs.bg_1_pattern_addr(), height);
        }
        if !self.tile_maps[0].check_and_set_addr(regs.bg1_settings) {
            self.tile_maps[0] = TileMap::new(&self.device, regs.bg1_settings, regs.bg_1_large_tiles());
        }

        if self.pattern_mem[1].get_start_addr() != regs.bg_2_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg_2_pattern_addr(), self.pattern_mem[1].get_bits_per_pixel() as u32);
            self.pattern_mem[1].set_addr(regs.bg_2_pattern_addr(), height);
        }
        if !self.tile_maps[1].check_and_set_addr(regs.bg2_settings) {
            self.tile_maps[1] = TileMap::new(&self.device, regs.bg2_settings, regs.bg_2_large_tiles());
        }

        if self.pattern_mem[2].get_start_addr() != regs.bg_3_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg_3_pattern_addr(), self.pattern_mem[2].get_bits_per_pixel() as u32);
            self.pattern_mem[2].set_addr(regs.bg_3_pattern_addr(), height);
        }
        if !self.tile_maps[2].check_and_set_addr(regs.bg3_settings) {
            self.tile_maps[2] = TileMap::new(&self.device, regs.bg3_settings, regs.bg_3_large_tiles());
        }

        if self.pattern_mem[3].get_start_addr() != regs.bg_4_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg_4_pattern_addr(), self.pattern_mem[3].get_bits_per_pixel() as u32);
            self.pattern_mem[3].set_addr(regs.bg_4_pattern_addr(), height);
        }
        if !self.tile_maps[3].check_and_set_addr(regs.bg4_settings) {
            self.tile_maps[3] = TileMap::new(&self.device, regs.bg4_settings, regs.bg_4_large_tiles());
        }

        // Check data dirtiness
        if mem.is_vram_dirty() {
            self.pattern_mem[0].clear_image(&mem);
            self.pattern_mem[1].clear_image(&mem);
            self.pattern_mem[2].clear_image(&mem);
            self.pattern_mem[3].clear_image(&mem);

            // Clear tile maps.
            self.tile_maps[0].update(&mem);
            self.tile_maps[1].update(&mem);
            self.tile_maps[2].update(&mem);
            self.tile_maps[3].update(&mem);

            mem.vram_reset_dirty_range();
        }

        // Check OAM dirtiness

        // Check CGRAM dirtiness
    }

    // Retrieve structures.
    // Get texture for a bg.
    pub fn get_bg_image(&mut self, bg_num: usize) -> (PatternImage, PatternFuture) {
        let mem = self.native_mem.lock().expect("Couldn't lock native mem.");
        self.pattern_mem[bg_num].get_image(&mem)
    }

    // Get vertices for a line on a bg.
    pub fn get_bg_lo_vertices(&mut self, bg_num: usize, y: u8) -> Option<VertexBuffer> {
        // TODO: check mode?
        self.tile_maps[bg_num].get_lo_vertex_buffer(y)
    }

    pub fn get_bg_hi_vertices(&mut self, bg_num: usize, y: u8) -> Option<VertexBuffer> {
        // TODO: check mode?
        self.tile_maps[bg_num].get_hi_vertex_buffer(y)
    }
}

// Internal
impl MemoryCache {
    // Switch mode: setup backgrounds. // TODO: other stuff here?
    fn switch_mode(&mut self, mode: u8) {
        match mode {
            0 => {
                self.pattern_mem[0] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0);
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0);
                }
            },
            1 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4, 0);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4, 0);
                }
            },
            2 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4, 0);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4, 0);
                }
            },
            3 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_8, 0);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4, 0);
                }
            },
            4 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_8, 0);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0);
                }
            },
            5 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4, 0);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2, 0);
                }
            },
            6 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, &self.device, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4, 0);
                }
            },
            7 => {
                panic!("Mode 7 not supported!");
            },
            _ => unreachable!()
        }
    }
}