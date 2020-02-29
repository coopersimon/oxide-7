// Drawing.

use crate::video::{
    VideoMem,
    ram::{
        BGReg,
        ObjectSettings,
        SpritePriority
    },
    render::{
        Colour,
        VideoMode,
        patternmem::{
            BitsPerPixel,
            PatternMem
        },
        bgcache::{
            BGCache,
            TileAttributes
        },
        palette::PaletteMem
    }
};

const SCREEN_WIDTH: usize = 256;

pub struct Renderer {
    // caches...
    mode: VideoMode,

    bg_pattern_mem: [PatternMem; 4],
    bg_cache: [BGCache; 4],

    obj_pattern_mem: [PatternMem; 2],

    palettes: PaletteMem
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            mode: VideoMode::_7,

            bg_pattern_mem: [
                PatternMem::new(BitsPerPixel::_2),
                PatternMem::new(BitsPerPixel::_2),
                PatternMem::new(BitsPerPixel::_2),
                PatternMem::new(BitsPerPixel::_2)
            ],
            bg_cache: [
                BGCache::new(BGReg::default(), false),
                BGCache::new(BGReg::default(), false),
                BGCache::new(BGReg::default(), false),
                BGCache::new(BGReg::default(), false),
            ],

            obj_pattern_mem: [
                PatternMem::new(BitsPerPixel::_4),
                PatternMem::new(BitsPerPixel::_4),
            ],

            palettes: PaletteMem::new()
        }
    }

    pub fn draw_line(&mut self, mem: &mut VideoMem, target: &mut [u8], y: usize) {
        if !mem.get_bg_registers().in_fblank() {
            // Refresh caches
            self.setup_caches(mem);

            match self.mode {
                VideoMode::_0 => self.draw_line_mode_0(mem, target, y),
                VideoMode::_1 => self.draw_line_mode_1(mem, target, y),
                VideoMode::_2 => panic!("Mode 2 not supported."),
                VideoMode::_3 => panic!("Mode 3 not supported."),
                VideoMode::_4 => panic!("Mode 4 not supported."),
                VideoMode::_5 => panic!("Mode 5 not supported."),
                VideoMode::_6 => panic!("Mode 6 not supported."),
                VideoMode::_7 => panic!("Mode 7 not supported."),
            }
        }
    }
}

// Caches
impl Renderer {
    fn setup_caches(&mut self, mem: &mut VideoMem) {
        // Check mode and alter backgrounds.
        let stored_mode = VideoMode::from(mem.get_bg_registers().get_mode());
        if stored_mode != self.mode {
            self.switch_mode(stored_mode);
        }

        let mut recreate_borders = false;
        let num_bgs = self.num_bgs();

        // Check background mem locations
        let regs = mem.get_bg_registers();
        for (bg, (bg_pattern, cache)) in self.bg_pattern_mem.iter_mut().zip(self.bg_cache.iter_mut()).take(num_bgs).enumerate() {
            if bg_pattern.get_start_addr() != regs.bg_pattern_addr(bg) {
                let height = regs.get_pattern_table_height(regs.bg_pattern_addr(bg), bg_pattern.get_bits_per_pixel() as u32);
                bg_pattern.set_addr(regs.bg_pattern_addr(bg), height as u16);    // TODO: figure out this u32, u16 mess
                recreate_borders = true;
            }
            if cache.check_if_valid(regs.get_bg_settings(bg), regs.bg_large_tiles(bg)) {
                *cache = BGCache::new(regs.get_bg_settings(bg), regs.bg_large_tiles(bg));
                recreate_borders = true;
            }
        }

        // Check OAM dirtiness.
        //self.obj_mem.check_and_set_obj_settings(regs.get_object_settings());
        if self.obj_pattern_mem[0].get_start_addr() != regs.obj0_pattern_addr() {
            self.obj_pattern_mem[0].set_addr_obj(regs.obj0_pattern_addr());
            recreate_borders = true;
        }
        if self.obj_pattern_mem[1].get_start_addr() != regs.objn_pattern_addr() {
            self.obj_pattern_mem[1].set_addr_obj(regs.objn_pattern_addr());
            recreate_borders = true;
        }

        // If borders have changed, reset in vram.
        if recreate_borders {
            let borders = regs.get_vram_borders();
            mem.vram_set_borders(&borders);
        }

        // If vram is dirty:
        for (bg_pattern, cache) in self.bg_pattern_mem.iter_mut().zip(self.bg_cache.iter_mut()).take(num_bgs) {
            let tiles_changed = if mem.vram_is_dirty(bg_pattern.get_start_addr()) {
                bg_pattern.make_tiles(mem.get_vram());
                true
            } else { false };
            cache.construct(&bg_pattern, &mem, tiles_changed);
        }

        for obj_pattern in self.obj_pattern_mem.iter_mut() {
            if mem.vram_is_dirty(obj_pattern.get_start_addr()) {
                obj_pattern.make_tiles(mem.get_vram());
            }
        }

        mem.vram_reset_dirty_range();

        // Check CGRAM dirtiness
        if mem.is_cgram_bg_dirty() {
            self.palettes.make_bg_palette(mem);
        }
        if mem.is_cgram_obj_dirty() {
            self.palettes.make_obj_palette(mem);
        }
        mem.cgram_reset_dirty();
    }

    // Switch mode: setup backgrounds. // TODO: other stuff here?
    fn switch_mode(&mut self, mode: VideoMode) {
        use VideoMode::*;

        self.mode = mode;
        match mode {
            _0 => {
                self.bg_pattern_mem[0] = PatternMem::new(BitsPerPixel::_2);
                if self.bg_pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.bg_pattern_mem[1] = PatternMem::new(BitsPerPixel::_2);
                }
            },
            _1 => {
                if self.bg_pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.bg_pattern_mem[0] = PatternMem::new(BitsPerPixel::_4);
                }
                if self.bg_pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.bg_pattern_mem[1] = PatternMem::new(BitsPerPixel::_4);
                }
            },
            _2 => {
                if self.bg_pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.bg_pattern_mem[0] = PatternMem::new(BitsPerPixel::_4);
                }
                if self.bg_pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.bg_pattern_mem[1] = PatternMem::new(BitsPerPixel::_4);
                }
            },
            _3 => {
                if self.bg_pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.bg_pattern_mem[0] = PatternMem::new(BitsPerPixel::_8);
                }
                if self.bg_pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.bg_pattern_mem[1] = PatternMem::new(BitsPerPixel::_4);
                }
            },
            _4 => {
                if self.bg_pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_8 {
                    self.bg_pattern_mem[0] = PatternMem::new(BitsPerPixel::_8);
                }
                if self.bg_pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.bg_pattern_mem[1] = PatternMem::new(BitsPerPixel::_2);
                }
            },
            _5 => {
                if self.bg_pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.bg_pattern_mem[0] = PatternMem::new(BitsPerPixel::_4);
                }
                if self.bg_pattern_mem[1].get_bits_per_pixel() != BitsPerPixel::_2 {
                    self.bg_pattern_mem[1] = PatternMem::new(BitsPerPixel::_2);
                }
            },
            _6 => {
                if self.bg_pattern_mem[0].get_bits_per_pixel() != BitsPerPixel::_4 {
                    self.bg_pattern_mem[0] = PatternMem::new(BitsPerPixel::_4);
                }
            },
            _7 => {
                panic!("Mode 7 not supported!");
            }
        }
    }

    // Get the number of backgrounds active
    fn num_bgs(&self) -> usize {
        use VideoMode::*;
        match self.mode {
            _0 => 4,
            _1 => 3,
            _ => 2
        }
    }
}

// Drawing types
#[derive(Clone, Copy)]
enum SpritePixel {
    Prio3(Colour),
    Prio2(Colour),
    Prio1(Colour),
    Prio0(Colour),
    None
}

enum BGPixel {
    Hi(Colour), // Priority bit set
    Lo(Colour), // Priority bit clear
    None        // Transparent
}

impl BGPixel {
    #[inline]
    fn any(self) -> Option<Colour> {
        match self {
            BGPixel::Hi(c) => Some(c),
            BGPixel::Lo(c) => Some(c),
            BGPixel::None => None
        }
    }
}

enum BG3Pixel {
    XHi(Colour),    // Priority bit set in tile and reg
    Hi(Colour),     // Priority bit set in tile
    Lo(Colour),     // Priority bit clear in tile
    None
}

impl BG3Pixel {
    #[inline]
    fn any(self) -> Option<Colour> {
        match self {
            BG3Pixel::XHi(c) => Some(c),
            BG3Pixel::Hi(c) => Some(c),
            BG3Pixel::Lo(c) => Some(c),
            BG3Pixel::None => None
        }
    }
}

#[inline]
fn write_pixel(output: &mut [u8], colour: Colour) {
    output[0] = colour.r;
    output[1] = colour.g;
    output[2] = colour.b;
}

// Generic drawing
impl Renderer {
    // TODO: lots of cleanup here
    fn draw_sprites_to_line(&self, mem: &VideoMem, line: &mut [SpritePixel], y: u8) {
        // TODO: get this elsewhere..?
        let obj_regs = ObjectSettings::from_bits_truncate(mem.get_bg_registers().get_object_settings());
        let (small, large) = match (obj_regs & ObjectSettings::SIZE).bits() >> 5 {
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

        let objects = mem.get_oam();
        
        objects.iter().filter(|object| { // See if this sprite should appear on this line.
            let size_y = if object.large {large.1} else {small.1};

            let bottom_y = object.y.wrapping_add(size_y - 1);
            
            if bottom_y > object.y {
                (y >= object.y) && (y <= bottom_y)
            } else {
                (y >= object.y) || (y <= bottom_y)
            }
        }).take(32).for_each(|object| { // Actually do drawing.
            let size = if object.large {large} else {small};
            let sprite_y = y - object.y;   // TODO: deal with wraparound.
            let y_pixel = if object.y_flip() {size.1 - 1 - sprite_y} else {sprite_y} as usize;

            for x in 0..size.0 {
                let line_x = object.x + x;
                if line_x >= 0 && line_x < 256 {  // TODO: no magic number here.
                    let x_pixel = if object.x_flip() {size.0 - 1 - x} else {x} as usize;
                    let tile_num = object.calc_tile_num(x_pixel, y_pixel);

                    let texel = self.obj_pattern_mem[object.name_table()]
                        .ref_tile(tile_num)
                        .get_texel(x_pixel % 8, y_pixel % 8) as usize;

                    if texel != 0 {
                        let colour = self.palettes.get_obj_colour(object.palette_offset() + texel);
                        line[line_x as usize] = match object.priority() {
                            SpritePriority::_3 => SpritePixel::Prio3(colour),
                            SpritePriority::_2 => SpritePixel::Prio2(colour),
                            SpritePriority::_1 => SpritePixel::Prio1(colour),
                            SpritePriority::_0 => SpritePixel::Prio0(colour),
                        };
                    }
                }
            }   // for sprite x pixels
        });
    }

    fn bg_pixel(&self, mem: &VideoMem, x: usize, y: usize, bg: usize, bpp: BitsPerPixel) -> BGPixel {
        let bg_x = (x + (mem.get_bg_registers().get_bg_scroll_x(bg) as usize)) % self.bg_cache[bg].width();
        let bg_y = (y + (mem.get_bg_registers().get_bg_scroll_y(bg) as usize)) % self.bg_cache[bg].height();

        let texel = self.bg_cache[bg].get_texel(bg_x, bg_y) as usize;

        if texel == 0 {
            BGPixel::None
        } else {
            let attrs = self.bg_cache[bg].get_attrs(bg_x, bg_y);
            let palette_shift = (bpp as usize) - 2;
            let palette_num = ((attrs & TileAttributes::PALETTE).bits() << palette_shift) as usize;
            let colour = self.palettes.get_bg_colour(palette_num + texel);
            if attrs.contains(TileAttributes::PRIORITY) {
                BGPixel::Hi(colour)
            } else {
                BGPixel::Lo(colour)
            }
        }
    }

    fn mode_1_bg_3(&self, mem: &VideoMem, x: usize, y: usize) -> BG3Pixel {
        const BG_3: usize = 2;
        let bg_x = (x + (mem.get_bg_registers().get_bg_scroll_x(BG_3) as usize)) % self.bg_cache[BG_3].width();
        let bg_y = (y + (mem.get_bg_registers().get_bg_scroll_y(BG_3) as usize)) % self.bg_cache[BG_3].height();

        let texel = self.bg_cache[BG_3].get_texel(bg_x, bg_y) as usize;

        if texel == 0 {
            BG3Pixel::None
        } else {
            let attrs = self.bg_cache[BG_3].get_attrs(bg_x, bg_y);
            let palette_num = (attrs & TileAttributes::PALETTE).bits() as usize;
            let colour = self.palettes.get_bg_colour(palette_num + texel);
            if attrs.contains(TileAttributes::PRIORITY) {
                if mem.get_bg_registers().get_bg3_priority() {
                    BG3Pixel::XHi(colour)
                } else {
                    BG3Pixel::Hi(colour)
                }
            } else {
                BG3Pixel::Lo(colour)
            }
        }
    }
}

// Drawing modes
impl Renderer {
    fn draw_line_mode_0(&self, mem: &VideoMem, target: &mut [u8], y: usize) {
        let target_start = y * SCREEN_WIDTH;

        let mut sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];

        self.draw_sprites_to_line(mem, &mut sprite_pixels, y as u8);

        for (x, i) in target.chunks_mut(4).skip(target_start).take(SCREEN_WIDTH).enumerate() {
            match sprite_pixels[x] {
                SpritePixel::Prio3(s3) => write_pixel(i, s3),
                SpritePixel::Prio2(s2) => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_2) {
                    BGPixel::Hi(b1) => write_pixel(i, b1),
                    _ => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_2) {
                        BGPixel::Hi(b2) => write_pixel(i, b2),
                        _ => write_pixel(i, s2)
                    }
                },
                SpritePixel::Prio1(s1) => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_2) {
                    BGPixel::Hi(b1) => write_pixel(i, b1),
                    BGPixel::Lo(b1) => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_2) {
                        BGPixel::Hi(b2) => write_pixel(i, b2),
                        _ => write_pixel(i, b1)
                    },
                    BGPixel::None => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_2).any() {
                        Some(b2) => write_pixel(i, b2),
                        None => write_pixel(i, s1)
                    }
                },
                SpritePixel::Prio0(s0) => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_2) {
                    BGPixel::Hi(b1) => write_pixel(i, b1),
                    BGPixel::Lo(b1) => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_2) {
                        BGPixel::Hi(b2) => write_pixel(i, b2),
                        _ => write_pixel(i, b1)
                    },
                    BGPixel::None => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_2).any() {
                        Some(b2) => write_pixel(i, b2),
                        None => match self.bg_pixel(mem, x, y, 2, BitsPerPixel::_2) {
                            BGPixel::Hi(b3) => write_pixel(i, b3),
                            _ => match self.bg_pixel(mem, x, y, 3, BitsPerPixel::_2) {
                                BGPixel::Hi(b4) => write_pixel(i, b4),
                                _ => write_pixel(i, s0)
                            }
                        }
                    }
                },
                SpritePixel::None => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_2) {
                    BGPixel::Hi(b1) => write_pixel(i, b1),
                    BGPixel::Lo(b1) => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_2) {
                        BGPixel::Hi(b2) => write_pixel(i, b2),
                        _ => write_pixel(i, b1)
                    },
                    BGPixel::None => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_2).any() {
                        Some(b2) => write_pixel(i, b2),
                        None => match self.bg_pixel(mem, x, y, 2, BitsPerPixel::_2) {
                            BGPixel::Hi(b3) => write_pixel(i, b3),
                            BGPixel::Lo(b3) => match self.bg_pixel(mem, x, y, 3, BitsPerPixel::_2) {
                                BGPixel::Hi(b4) => write_pixel(i, b4),
                                _ => write_pixel(i, b3)
                            },
                            BGPixel::None => match self.bg_pixel(mem, x, y, 3, BitsPerPixel::_2).any() {
                                Some(b4) => write_pixel(i, b4),
                                _ => write_pixel(i, mem.get_window_registers().get_fixed_colour())
                            }
                        }
                    }
                },
            }
        }
    }

    fn draw_line_mode_1(&self, mem: &VideoMem, target: &mut [u8], y: usize) {
        let target_start = y * SCREEN_WIDTH;

        let mut sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];

        self.draw_sprites_to_line(mem, &mut sprite_pixels, y as u8);

        for (x, i) in target.chunks_mut(4).skip(target_start).take(SCREEN_WIDTH).enumerate() {
            match sprite_pixels[x] {
                SpritePixel::Prio3(s3) => match self.mode_1_bg_3(mem, x, y) {
                    BG3Pixel::XHi(b3) => write_pixel(i, b3),
                    _ => write_pixel(i, s3)
                },
                SpritePixel::Prio2(s2) => match self.mode_1_bg_3(mem, x, y) {
                    BG3Pixel::XHi(b3) => write_pixel(i, b3),
                    _ => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_4) {
                        BGPixel::Hi(b1) => write_pixel(i, b1),
                        _ => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_4) {
                            BGPixel::Hi(b2) => write_pixel(i, b2),
                            _ => write_pixel(i, s2)
                        }
                    }
                },
                SpritePixel::Prio1(s1) => match self.mode_1_bg_3(mem, x, y) {
                    BG3Pixel::XHi(b3) => write_pixel(i, b3),
                    _ => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_4) {
                        BGPixel::Hi(b1) => write_pixel(i, b1),
                        BGPixel::Lo(b1) => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_4) {
                            BGPixel::Hi(b2) => write_pixel(i, b2),
                            _ => write_pixel(i, b1)
                        },
                        BGPixel::None => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_4).any() {
                            Some(b2) => write_pixel(i, b2),
                            None => write_pixel(i, s1)
                        }
                    }
                },
                SpritePixel::Prio0(s0) => {
                    let bg3_pixel = self.mode_1_bg_3(mem, x, y);
                    match bg3_pixel {
                        BG3Pixel::XHi(b3) => write_pixel(i, b3),
                        _ => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_4) {
                            BGPixel::Hi(b1) => write_pixel(i, b1),
                            BGPixel::Lo(b1) => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_4) {
                                BGPixel::Hi(b2) => write_pixel(i, b2),
                                _ => write_pixel(i, b1)
                            },
                            BGPixel::None => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_4).any() {
                                Some(b2) => write_pixel(i, b2),
                                None => match bg3_pixel {
                                    BG3Pixel::Hi(b3) => write_pixel(i, b3),
                                    _ => write_pixel(i, s0)
                                }
                            }
                        }
                    }
                },
                SpritePixel::None => {
                    let bg3_pixel = self.mode_1_bg_3(mem, x, y);
                    match bg3_pixel {
                        BG3Pixel::XHi(b3) => write_pixel(i, b3),
                        _ => match self.bg_pixel(mem, x, y, 0, BitsPerPixel::_4) {
                            BGPixel::Hi(b1) => write_pixel(i, b1),
                            BGPixel::Lo(b1) => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_4) {
                                BGPixel::Hi(b2) => write_pixel(i, b2),
                                _ => write_pixel(i, b1)
                            },
                            BGPixel::None => match self.bg_pixel(mem, x, y, 1, BitsPerPixel::_4).any() {
                                Some(b2) => write_pixel(i, b2),
                                None => match bg3_pixel.any() {
                                    Some(b3) => write_pixel(i, b3),
                                    _ => write_pixel(i, mem.get_window_registers().get_fixed_colour())
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}