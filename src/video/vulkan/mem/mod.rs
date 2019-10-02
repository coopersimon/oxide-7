// Converting native VRAM, CGRAM and OAM into Vulkan structures.

pub mod palette;
pub mod patternmem;
mod sprite;
mod tilemap;

use vulkano::{
    device::{
        Device,
        Queue
    }
};

use std::sync::Arc;

use crate::video::VRamRef;

use super::{
    uniforms::UniformCache,
    VertexBuffer,
    super::VideoMode
};

use patternmem::*;
use tilemap::*;
use sprite::*;
use palette::*;

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
    pattern_mem:    [PatternMem; 4],
    tile_maps:      [TileMap; 4],

    obj_mem:        SpriteMem,
    obj0_pattern:   PatternMem,
    objn_pattern:   PatternMem,

    palette:        Palette,

    // Vulkan things
    device:         Arc<Device>,
    queue:          Arc<Queue>,
    uniform_cache:  UniformCache,
}

impl MemoryCache {
    pub fn new(vram: VRamRef, device: &Arc<Device>, queue: &Arc<Queue>, uniform_cache: UniformCache) -> Self {
        let pattern_mem = [
            PatternMem::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2),  // BG 1 can be 2, 4 or 8 BPP
            PatternMem::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2),  // BG 2 can be 2, 4 or 7 BPP
            PatternMem::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2),  // BG 3 can only be 2 BPP
            PatternMem::new(queue, PATTERN_WIDTH, 0, BitsPerPixel::_2)   // BG 4 can only be 2 BPP
        ];

        let tile_maps = [
            TileMap::new(device, 0, false),
            TileMap::new(device, 0, false),
            TileMap::new(device, 0, false),
            TileMap::new(device, 0, false)
        ];

        MemoryCache {
            native_mem:     vram,

            mode:           VideoMode::_7,
            bg3_priority:   false,

            pattern_mem:    pattern_mem,
            tile_maps:      tile_maps,

            obj_mem:        SpriteMem::new(device),
            obj0_pattern:   PatternMem::new(queue, PATTERN_WIDTH, PATTERN_WIDTH, BitsPerPixel::_4),
            objn_pattern:   PatternMem::new(queue, PATTERN_WIDTH, PATTERN_WIDTH, BitsPerPixel::_4),

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

        let mut mem = self.native_mem.borrow_mut();

        // Check background mem locations
        let regs = mem.get_registers();
        if self.pattern_mem[0].get_start_addr() != regs.bg1_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg1_pattern_addr(), self.pattern_mem[0].get_bits_per_pixel() as u32);
            self.pattern_mem[0].set_addr(regs.bg1_pattern_addr(), height);
        }
        if !self.tile_maps[0].check_and_set_addr(regs.bg1_settings, regs.bg1_large_tiles()) {
            self.tile_maps[0] = TileMap::new(&self.device, regs.bg1_settings, regs.bg1_large_tiles());
        }

        if self.pattern_mem[1].get_start_addr() != regs.bg2_pattern_addr() {
            let height = regs.get_pattern_table_height(regs.bg2_pattern_addr(), self.pattern_mem[1].get_bits_per_pixel() as u32);
            self.pattern_mem[1].set_addr(regs.bg2_pattern_addr(), height);
        }
        if !self.tile_maps[1].check_and_set_addr(regs.bg2_settings, regs.bg2_large_tiles()) {
            self.tile_maps[1] = TileMap::new(&self.device, regs.bg2_settings, regs.bg2_large_tiles());
        }

        if (stored_mode == VideoMode::_1) || (stored_mode == VideoMode::_0) {
            if self.pattern_mem[2].get_start_addr() != regs.bg3_pattern_addr() {
                let height = regs.get_pattern_table_height(regs.bg3_pattern_addr(), self.pattern_mem[2].get_bits_per_pixel() as u32);
                self.pattern_mem[2].set_addr(regs.bg3_pattern_addr(), height);
            }
            if !self.tile_maps[2].check_and_set_addr(regs.bg3_settings, regs.bg3_large_tiles()) {
                self.tile_maps[2] = TileMap::new(&self.device, regs.bg3_settings, regs.bg3_large_tiles());
            }
        }

        if stored_mode == VideoMode::_0 {
            if self.pattern_mem[3].get_start_addr() != regs.bg4_pattern_addr() {
                let height = regs.get_pattern_table_height(regs.bg4_pattern_addr(), self.pattern_mem[3].get_bits_per_pixel() as u32);
                self.pattern_mem[3].set_addr(regs.bg4_pattern_addr(), height);
            }
            if !self.tile_maps[3].check_and_set_addr(regs.bg4_settings, regs.bg4_large_tiles()) {
                self.tile_maps[3] = TileMap::new(&self.device, regs.bg4_settings, regs.bg4_large_tiles());
            }
        }

        // Check OAM dirtiness (always recreate for now TODO: caching of object vertices)
        self.obj_mem.check_and_set_obj_settings(regs.get_object_settings());
        if self.obj0_pattern.get_start_addr() != regs.obj0_pattern_addr() {
            self.obj0_pattern.set_addr(regs.obj0_pattern_addr(), PATTERN_WIDTH);
        }
        if self.objn_pattern.get_start_addr() != regs.objn_pattern_addr() {
            self.objn_pattern.set_addr(regs.objn_pattern_addr(), PATTERN_WIDTH);
        }

        self.bg3_priority = regs.get_bg3_priority();

        // Check data dirtiness
        if mem.is_vram_dirty() {
            self.pattern_mem[0].clear_image(&mem);
            self.pattern_mem[1].clear_image(&mem);
            self.pattern_mem[2].clear_image(&mem);
            self.pattern_mem[3].clear_image(&mem);

            self.obj0_pattern.clear_image(&mem);
            self.objn_pattern.clear_image(&mem);

            // Clear tile maps.
            self.tile_maps[0].update(&mem);
            self.tile_maps[1].update(&mem);
            self.tile_maps[2].update(&mem);
            self.tile_maps[3].update(&mem);

            mem.vram_reset_dirty_range();
        }

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
    // Get texture for a bg.
    pub fn get_bg_image(&mut self, bg_num: usize) -> (ImageDescriptorSet, Option<PatternFuture>) {
        let mem = self.native_mem.borrow();
        self.pattern_mem[bg_num].get_image(&mem, &mut self.uniform_cache, true)
    }

    // Get vertices for a line on a bg.
    pub fn get_bg_lo_vertices(&mut self, bg_num: usize, y: u16) -> Option<VertexBuffer> {
        self.tile_maps[bg_num].get_lo_vertex_buffer(y)
    }

    pub fn get_bg_hi_vertices(&mut self, bg_num: usize, y: u16) -> Option<VertexBuffer> {
        self.tile_maps[bg_num].get_hi_vertex_buffer(y)
    }

    // Get texture for sprites.
    pub fn get_sprite_image_0(&mut self) -> (ImageDescriptorSet, Option<PatternFuture>) {
        let mem = self.native_mem.borrow();
        self.obj0_pattern.get_image(&mem, &mut self.uniform_cache, false)
    }

    pub fn get_sprite_image_n(&mut self) -> (ImageDescriptorSet, Option<PatternFuture>) {
        let mem = self.native_mem.borrow();
        self.objn_pattern.get_image(&mem, &mut self.uniform_cache, false)
    }

    // Get vertices for a line of sprites.
    pub fn get_sprite_vertices_0(&mut self, priority: usize, y: u16) -> Option<VertexBuffer> {
        let mut mem = self.native_mem.borrow_mut();
        let (oam_hi, oam_lo) = mem.get_oam();
        self.obj_mem.get_vertex_buffer_0(priority, y as u8, oam_hi, oam_lo)
    }

    pub fn get_sprite_vertices_n(&mut self, priority: usize, y: u16) -> Option<VertexBuffer> {
        let mut mem = self.native_mem.borrow_mut();
        let (oam_hi, oam_lo) = mem.get_oam();
        self.obj_mem.get_vertex_buffer_n(priority, y as u8, oam_hi, oam_lo)
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

    pub fn get_bg_push_constants(&self, bg_num: usize) -> super::BGPushConstants {
        let atlas_size_pixels = self.pattern_mem[bg_num].get_size();
        let tile_height = self.tile_maps[bg_num].get_tile_height() as f32;

        let tex_size_x = tile_height / atlas_size_pixels.0;
        let tex_size_y = tile_height / atlas_size_pixels.1;

        let atlas_size_tiles = [atlas_size_pixels.0 / tile_height, atlas_size_pixels.1 / tile_height];

        let map_size = self.tile_maps[bg_num].get_map_size();

        let vertex_offset_x = (self.get_scroll_x(bg_num) as f32) * SCROLL_X_FRAC;
        let vertex_offset_y = (self.get_scroll_y(bg_num) as f32) * SCROLL_Y_FRAC;

        let pc = super::BGPushConstants {
            tex_size:           [tex_size_x, tex_size_y],     // X: 1/16 for 8x8 tile, 1/8 for 16x16 tile
            atlas_size:         atlas_size_tiles,
            tile_size:          [tile_height / 128.0, 1.0 / 112.0],
            map_size:           [map_size.0, map_size.1],
            vertex_offset:      [vertex_offset_x, vertex_offset_y],
            palette_offset:     0,  // TODO: offset for each bg for mode 0? (32 per bg)
            palette_size:       1 << (self.pattern_mem[bg_num].get_bits_per_pixel() as u32),
            tex_pixel_height:   tile_height,
        };

        //println!("PC for {}, {:?}", bg_num, pc);

        pc
    }

    pub fn get_obj_push_constants(&self) -> super::ObjPushConstants {
        super::ObjPushConstants {
            small_tex_size: self.obj_mem.get_small_tex_size(),
            large_tex_size: self.obj_mem.get_large_tex_size()
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
                self.pattern_mem[0] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                }
            },
            _1 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _2 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _3 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_8);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _4 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_8);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                }
            },
            _5 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
                if self.pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.pattern_mem[1] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_2);
                }
            },
            _6 => {
                if self.pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.pattern_mem[0] = PatternMem::new(&self.queue, PATTERN_WIDTH, PATTERN_HEIGHT, BitsPerPixel::_4);
                }
            },
            _7 => {
                panic!("Mode 7 not supported!");
            }
        }
    }
}