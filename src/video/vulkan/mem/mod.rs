// Converting native VRAM, CGRAM and OAM into Vulkan structures.

pub mod palette;
pub mod texturecache;
mod sprite;
mod tilemap;

use vulkano::{
    buffer::ImmutableBuffer,
    device::{
        Device,
        Queue
    }
};

use std::sync::Arc;

use crate::video::{
    VRamRef,
    patternmem::BitsPerPixel
};

use super::{
    uniforms::UniformCache,
    VertexBuffer,
    super::VideoMode,
    super::ram::BGReg
};

use tilemap::*;
use sprite::*;
use palette::*;
use texturecache::*;

const PATTERN_WIDTH: u32 = 16 * 8; // Pattern width in pixels (16 tiles)
const PATTERN_HEIGHT: u32 = 64 * 8; // Pattern height in pixels (16 tiles)

const VIEW_WIDTH: usize = 256;           // Width of visible area in pixels.
const VIEW_HEIGHT: usize = 224;          // Height of visible area in pixels.

const SCROLL_X_FRAC: f32 = -2.0 / VIEW_WIDTH as f32;   // Multiply scroll x by this to get vertex offset.
const SCROLL_Y_FRAC: f32 = -2.0 / VIEW_HEIGHT as f32;   // Multiply scroll y by this to get vertex offset.

pub struct MemoryCache {
    native_mem:     VRamRef,
    // Internal settings
    mode:           VideoMode,
    bg3_priority:   bool,

    // Internal mem
    pattern_mem:    [BGTexCache; 4],
    tile_maps:      [TileMap; 4],

    obj_mem:        SpriteMem,
    obj_pattern:    ObjTexCache,

    palette:        Palette,

    // Vulkan things
    device:         Arc<Device>,
    queue:          Arc<Queue>,
    uniform_cache:  UniformCache,
}

impl MemoryCache {
    pub fn new(vram: VRamRef, device: &Arc<Device>, queue: &Arc<Queue>, uniform_cache: UniformCache) -> Self {
        let pattern_mem = [
            BGTexCache::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2),  // BG 1 can be 2, 4 or 8 BPP
            BGTexCache::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2),  // BG 2 can be 2, 4 or 7 BPP
            BGTexCache::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2),  // BG 3 can only be 2 BPP
            BGTexCache::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2)   // BG 4 can only be 2 BPP
        ];

        let mut tile_maps = [
            TileMap::new(device, queue, BGReg::default(), false),
            TileMap::new(device, queue, BGReg::default(), false),
            TileMap::new(device, queue, BGReg::default(), false),
            TileMap::new(device, queue, BGReg::default(), false)
        ];

        for map in tile_maps.iter_mut() {
            map.update(&vram.borrow());
        }

        MemoryCache {
            native_mem:     vram,

            mode:           VideoMode::_7,
            bg3_priority:   false,

            pattern_mem:    pattern_mem,
            tile_maps:      tile_maps,

            obj_mem:        SpriteMem::new(device),
            obj_pattern:    ObjTexCache::new(device, queue),

            palette:        Palette::new(device),

            device:         device.clone(),
            queue:          queue.clone(),
            uniform_cache:  uniform_cache
        }
    }

    // Called every line. Checks mode and dirtiness of video memory.
    pub fn init(&mut self) {
        // Check mode and alter backgrounds.
        let stored_mode = VideoMode::from(self.native_mem.borrow().get_registers().get_mode());
        if stored_mode != self.mode {
            self.switch_mode(stored_mode);
        }

        let mut recreate_borders = false;
        let mut mem = self.native_mem.borrow_mut();

        // Check background mem locations
        let regs = mem.get_registers();
        if self.pattern_mem[0].get_start_addr() != regs.bg1_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg1_pattern_addr(), self.pattern_mem[0].get_bits_per_pixel() as u32);
            self.pattern_mem[0].set_addr(regs.bg1_pattern_addr(), height);
            recreate_borders = true;
        }
        if !self.tile_maps[0].check_and_set_addr(regs.get_bg1_settings(), regs.bg1_large_tiles()) {
            self.tile_maps[0] = TileMap::new(&self.device, &self.queue, regs.get_bg1_settings(), regs.bg1_large_tiles());
            recreate_borders = true;
        }

        if self.pattern_mem[1].get_start_addr() != regs.bg2_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg2_pattern_addr(), self.pattern_mem[1].get_bits_per_pixel() as u32);
            self.pattern_mem[1].set_addr(regs.bg2_pattern_addr(), height);
            recreate_borders = true;
        }
        if !self.tile_maps[1].check_and_set_addr(regs.get_bg2_settings(), regs.bg2_large_tiles()) {
            self.tile_maps[1] = TileMap::new(&self.device, &self.queue, regs.get_bg2_settings(), regs.bg2_large_tiles());
            recreate_borders = true;
        }

        if (stored_mode == VideoMode::_1) || (stored_mode == VideoMode::_0) {
            if self.pattern_mem[2].get_start_addr() != regs.bg3_pattern_addr() {
                let height = regs.get_pattern_table_height(regs.bg3_pattern_addr(), self.pattern_mem[2].get_bits_per_pixel() as u32);
                self.pattern_mem[2].set_addr(regs.bg3_pattern_addr(), height);
                recreate_borders = true;
            }
            if !self.tile_maps[2].check_and_set_addr(regs.get_bg3_settings(), regs.bg3_large_tiles()) {
                self.tile_maps[2] = TileMap::new(&self.device, &self.queue, regs.get_bg3_settings(), regs.bg3_large_tiles());
                recreate_borders = true;
            }
        }

        if stored_mode == VideoMode::_0 {
            if self.pattern_mem[3].get_start_addr() != regs.bg4_pattern_addr() {
                let height = regs.get_pattern_table_height(regs.bg4_pattern_addr(), self.pattern_mem[3].get_bits_per_pixel() as u32);
                self.pattern_mem[3].set_addr(regs.bg4_pattern_addr(), height);
                recreate_borders = true;
            }
            if !self.tile_maps[3].check_and_set_addr(regs.get_bg4_settings(), regs.bg4_large_tiles()) {
                self.tile_maps[3] = TileMap::new(&self.device, &self.queue, regs.get_bg4_settings(), regs.bg4_large_tiles());
                recreate_borders = true;
            }
        }

        // Check OAM dirtiness (always recreate for now TODO: caching of object vertices)
        self.obj_mem.check_and_set_obj_settings(regs.get_object_settings());
        if self.obj_pattern.get_start_addr_0() != regs.obj0_pattern_addr() {
            self.obj_pattern.set_addr_0(regs.obj0_pattern_addr());
            recreate_borders = true;
        }
        if self.obj_pattern.get_start_addr_n() != regs.objn_pattern_addr() {
            self.obj_pattern.set_addr_n(regs.objn_pattern_addr());
            recreate_borders = true;
        }

        self.bg3_priority = regs.get_bg3_priority();

        // If borders have changed, reset in vram.
        if recreate_borders {
            let borders = regs.get_vram_borders();
            mem.vram_set_borders(&borders);
        }

        // If vram is dirty:
        self.pattern_mem[0].clear_image(&mem);
        self.pattern_mem[1].clear_image(&mem);
        self.pattern_mem[2].clear_image(&mem);
        self.pattern_mem[3].clear_image(&mem);

        self.obj_pattern.clear_images(&mem);

        // Clear tile maps.
        self.tile_maps[0].update(&mem);
        self.tile_maps[1].update(&mem);
        self.tile_maps[2].update(&mem);
        self.tile_maps[3].update(&mem);

        mem.vram_reset_dirty_range();

        // Check CGRAM dirtiness
        if mem.is_cgram_bg_dirty() {
            self.palette.create_bg_buffer(&mut mem, &mut self.uniform_cache);

            if mem.is_cgram_obj_dirty() {
                self.palette.create_obj_buffer(&mut mem, &mut self.uniform_cache);
            }

            mem.cgram_reset_dirty();
        }
    }

    pub fn in_fblank(&self) -> bool {
        self.native_mem.borrow().get_registers().in_fblank()
    }

    // Retrieve structures.
    // Get texture for a BG.
    pub fn get_bg_image(&mut self, bg_num: usize) -> (BGImageDescriptorSet, Option<PatternFuture>) {
        let mem = self.native_mem.borrow();
        self.pattern_mem[bg_num].get_image(&mem, &mut self.uniform_cache)
    }

    // Get vertices for a BG.
    pub fn get_bg_vertex_buffer(&mut self, bg_num: usize) -> VertexBuffer {
        self.tile_maps[bg_num].get_vertex_buffer()
    }

    // Get indices for a line of a BG.
    pub fn get_bg_index_buffer(&mut self, bg_num: usize, y: u16) -> Arc<ImmutableBuffer<[u32]>> {
        self.tile_maps[bg_num].get_index_buffer(y)
    }

    // Get texture for sprites.
    pub fn get_sprite_images(&mut self) -> (ObjImageDescriptorSet, Option<PatternFuture>) {
        let mem = self.native_mem.borrow();
        self.obj_pattern.get_images(&mem, &mut self.uniform_cache)
    }

    // Get vertices for a line of sprites.
    pub fn get_sprite_vertices(&mut self, y: u16) -> Option<VertexBuffer> {
        let mut mem = self.native_mem.borrow_mut();
        let (oam_hi, oam_lo) = mem.get_oam();
        self.obj_mem.get_vertex_buffer(y as u8, oam_hi, oam_lo)
    }

    // Get the palettes.
    // A buffer wrapped in a descriptor set.
    pub fn get_bg_palette_buffer(&self) -> PaletteDescriptorSet {
        self.palette.get_bg_palette_buffer()
    }

    pub fn get_obj_palette_buffer(&self) -> PaletteDescriptorSet {
        self.palette.get_obj_palette_buffer()
    }

    // Get registers
    pub fn get_scroll_y(&self, bg_num: usize) -> u16 {
        let mem = self.native_mem.borrow();
        let regs = mem.get_registers();
        match bg_num {
            0 => regs.get_bg1_scroll_y(),
            1 => regs.get_bg2_scroll_y(),
            2 => regs.get_bg3_scroll_y(),
            _ => regs.get_bg4_scroll_y()
        }
    }

    pub fn get_scroll_x(&self, bg_num: usize) -> u16 {
        let mem = self.native_mem.borrow();
        let regs = mem.get_registers();
        match bg_num {
            0 => regs.get_bg1_scroll_x(),
            1 => regs.get_bg2_scroll_x(),
            2 => regs.get_bg3_scroll_x(),
            _ => regs.get_bg4_scroll_x()
        }
    }

    // Calculate line to fetch
    pub fn calc_y_line(&self, bg_num: usize, y: u16) -> u16 {
        let scroll_y = self.get_scroll_y(bg_num);
        let height = self.tile_maps[bg_num].get_pixel_height();
        y.wrapping_add(scroll_y) % height
    }

    pub fn get_bg_push_constants(&self, bg_num: usize, depth: [f32; 2]) -> super::BGPushConstants {
        let atlas_size_pixels = self.pattern_mem[bg_num].get_size();
        let tile_height = self.tile_maps[bg_num].get_tile_height() as f32;

        let tex_size_x = tile_height / atlas_size_pixels.0;
        let tex_size_y = tile_height / atlas_size_pixels.1;

        let atlas_size_tiles = [atlas_size_pixels.0 / tile_height, atlas_size_pixels.1 / tile_height];

        let map_size = self.tile_maps[bg_num].get_map_size();

        let vertex_offset_x = (self.get_scroll_x(bg_num) as f32) * SCROLL_X_FRAC;
        let vertex_offset_y = (self.get_scroll_y(bg_num) as f32) * SCROLL_Y_FRAC;

        super::BGPushConstants {
            tex_size:           [tex_size_x, tex_size_y],     // X: 1/16 for 8x8 tile, 1/8 for 16x16 tile
            atlas_size:         atlas_size_tiles,
            tile_size:          [tile_height / 128.0, 1.0 / 112.0],
            map_size:           [map_size.0, map_size.1],
            vertex_offset:      [vertex_offset_x, vertex_offset_y],
            depth:              depth,
            palette_offset:     if self.mode == VideoMode::_0 { (bg_num * 32) as u32 } else { 0 },
            palette_size:       1 << (self.pattern_mem[bg_num].get_bits_per_pixel() as u32),
            tex_pixel_height:   tile_height,
        }
    }

    pub fn get_obj_push_constants(&self, depth: [f32; 4]) -> super::ObjPushConstants {
        super::ObjPushConstants {
            depth:          depth,
            small_tex_size: self.obj_mem.get_small_tex_size(),
            large_tex_size: self.obj_mem.get_large_tex_size(),
        }
    }

    pub fn get_mode(&self) -> VideoMode {
        VideoMode::from(self.mode)
    }

    pub fn get_bg3_priority(&self) -> bool {
        self.bg3_priority
    }
}

// Internal
impl MemoryCache {
    // Switch mode: setup backgrounds. // TODO: other stuff here?
    fn switch_mode(&mut self, mode: VideoMode) {
        use VideoMode::*;

        self.mode = mode;
        match mode {
            _0 => {
                self.pattern_mem[0] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                }
            },
            _1 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _2 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _3 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.pattern_mem[0] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_8);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _4 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.pattern_mem[0] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_8);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                }
            },
            _5 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                }
            },
            _6 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = BGTexCache::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _7 => {
                panic!("Mode 7 not supported!");
            }
        }
    }
}