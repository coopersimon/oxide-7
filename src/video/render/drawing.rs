// Drawing.

use crate::video::{
    VideoMem,
    ram::{
        BGReg,
        Registers,
        ObjectSettings,
        Screen,
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
            BGData,
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

    pub fn draw_line(&mut self, mem: &VideoMem, target: &mut [u8], y: usize) {
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

// Caches
impl Renderer {
    pub fn setup_caches(&mut self, mem: &mut VideoMem) {
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

// Drawing modes
impl Renderer {
    fn draw_line_mode_0(&self, mem: &VideoMem, target: &mut [u8], y: usize) {
        let window_regs = mem.get_window_registers();
        let target_start = y * SCREEN_WIDTH;

        let mut main_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        let mut sub_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        self.draw_sprites_to_line(mem, &mut main_sprite_pixels, &mut sub_sprite_pixels, y as u8);
        let mut main_bg1_pixels = [BGData::default(); SCREEN_WIDTH];
        let mut sub_bg1_pixels = [BGData::default(); SCREEN_WIDTH];
        self.draw_bg_to_line(mem, 0, &mut main_bg1_pixels, &mut sub_bg1_pixels, y);
        let mut main_bg2_pixels = [BGData::default(); SCREEN_WIDTH];
        let mut sub_bg2_pixels = [BGData::default(); SCREEN_WIDTH];
        self.draw_bg_to_line(mem, 1, &mut main_bg2_pixels, &mut sub_bg2_pixels, y);
        let mut main_bg3_pixels = [BGData::default(); SCREEN_WIDTH];
        let mut sub_bg3_pixels = [BGData::default(); SCREEN_WIDTH];
        self.draw_bg_to_line(mem, 2, &mut main_bg3_pixels, &mut sub_bg3_pixels, y);
        let mut main_bg4_pixels = [BGData::default(); SCREEN_WIDTH];
        let mut sub_bg4_pixels = [BGData::default(); SCREEN_WIDTH];
        self.draw_bg_to_line(mem, 3, &mut main_bg4_pixels, &mut sub_bg4_pixels, y);

        for (x, i) in target.chunks_mut(4).skip(target_start).take(SCREEN_WIDTH).enumerate() {
            let main = {
                let sprite_pix = main_sprite_pixels[x];
                let bg1_pix = main_bg1_pixels[x];
                let bg2_pix = main_bg2_pixels[x];
                let bg3_pix = main_bg3_pixels[x];
                let bg4_pix = main_bg4_pixels[x];
                self.eval_mode_0(sprite_pix, bg1_pix, bg2_pix, bg3_pix, bg4_pix)
            };
            let sub = if window_regs.use_subscreen() {
                let sprite_pix = sub_sprite_pixels[x];
                let bg1_pix = sub_bg1_pixels[x];
                let bg2_pix = sub_bg2_pixels[x];
                let bg3_pix = sub_bg3_pixels[x];
                let bg4_pix = main_bg4_pixels[x];
                self.eval_mode_0(sprite_pix, bg1_pix, bg2_pix, bg3_pix, bg4_pix).any()
                    .unwrap_or(window_regs.get_fixed_colour())
            } else {
                window_regs.get_fixed_colour()
            };

            let col = match main {
                Pixel::BG1(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 0, x as u8),
                Pixel::BG2(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 1, x as u8),
                Pixel::BG3(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 2, x as u8),
                Pixel::BG4(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 3, x as u8),
                Pixel::ObjHi(c) => mem.get_window_registers().calc_colour_math_obj(c, sub, x as u8),
                Pixel::ObjLo(c) => c,
                Pixel::None => mem.get_window_registers().calc_colour_math_backdrop(self.palettes.get_bg_colour(0), sub, x as u8),
            };

            write_pixel(i, col);
        }
    }

    fn draw_line_mode_1(&self, mem: &VideoMem, target: &mut [u8], y: usize) {
        let window_regs = mem.get_window_registers();
        let target_start = y * SCREEN_WIDTH;

        let mut main_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        let mut sub_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        self.draw_sprites_to_line(mem, &mut main_sprite_pixels, &mut sub_sprite_pixels, y as u8);
        let mut main_bg1_pixels = [BGData::default(); SCREEN_WIDTH];
        let mut sub_bg1_pixels = [BGData::default(); SCREEN_WIDTH];
        self.draw_bg_to_line(mem, 0, &mut main_bg1_pixels, &mut sub_bg1_pixels, y);
        let mut main_bg2_pixels = [BGData::default(); SCREEN_WIDTH];
        let mut sub_bg2_pixels = [BGData::default(); SCREEN_WIDTH];
        self.draw_bg_to_line(mem, 1, &mut main_bg2_pixels, &mut sub_bg2_pixels, y);
        let mut main_bg3_pixels = [BGData::default(); SCREEN_WIDTH];
        let mut sub_bg3_pixels = [BGData::default(); SCREEN_WIDTH];
        self.draw_bg_to_line(mem, 2, &mut main_bg3_pixels, &mut sub_bg3_pixels, y);

        for (x, i) in target.chunks_mut(4).skip(target_start).take(SCREEN_WIDTH).enumerate() {
            let main = {
                let sprite_pix = main_sprite_pixels[x];
                let bg1_pix = main_bg1_pixels[x];
                let bg2_pix = main_bg2_pixels[x];
                let bg3_pix = main_bg3_pixels[x];
                self.eval_mode_1(mem.get_bg_registers().get_bg3_priority(), sprite_pix, bg1_pix, bg2_pix, bg3_pix)
            };
            let sub = if window_regs.use_subscreen() {
                let sprite_pix = sub_sprite_pixels[x];
                let bg1_pix = sub_bg1_pixels[x];
                let bg2_pix = sub_bg2_pixels[x];
                let bg3_pix = sub_bg3_pixels[x];
                self.eval_mode_1(mem.get_bg_registers().get_bg3_priority(), sprite_pix, bg1_pix, bg2_pix, bg3_pix).any()
                    .unwrap_or(window_regs.get_fixed_colour())
            } else {
                window_regs.get_fixed_colour()
            };

            let col = match main {
                Pixel::BG1(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 0, x as u8),
                Pixel::BG2(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 1, x as u8),
                Pixel::BG3(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 2, x as u8),
                Pixel::BG4(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 3, x as u8),
                Pixel::ObjHi(c) => mem.get_window_registers().calc_colour_math_obj(c, sub, x as u8),
                Pixel::ObjLo(c) => c,
                Pixel::None => mem.get_window_registers().calc_colour_math_backdrop(self.palettes.get_bg_colour(0), sub, x as u8),
            };

            write_pixel(i, col);
        }
    }
}

// Drawing types
#[derive(Clone, Copy)]
struct SpriteColour {
    colour:     Colour, // The colour of the sprite
    col_math:   bool    // Should it participate in colour math
}

#[derive(Clone, Copy)]
enum SpritePixel {
    Prio3(SpriteColour),
    Prio2(SpriteColour),
    Prio1(SpriteColour),
    Prio0(SpriteColour),
    Masked,
    None
}

impl SpritePixel {
    #[inline]
    fn is_masked(&self) -> bool {
        match self {
            SpritePixel::Masked => true,
            _ => false
        }
    }

    fn pixel(&self) -> Pixel {
        match self {
            SpritePixel::Prio3(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            SpritePixel::Prio2(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            SpritePixel::Prio1(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            SpritePixel::Prio0(c) => if c.col_math {Pixel::ObjHi(c.colour)} else {Pixel::ObjLo(c.colour)},
            _ => panic!("Don't call this on pixels with no colour value.")
        }
    }
}

enum Pixel {
    BG1(Colour),
    BG2(Colour),
    BG3(Colour),
    BG4(Colour),
    ObjHi(Colour),  // Uses object palette 4-7, uses colour math
    ObjLo(Colour),  // Uses object palette 0-3, doesn't use colmath
    None
}

impl Pixel {
    fn any(self) -> Option<Colour> {
        match self {
            Pixel::BG1(c) => Some(c),
            Pixel::BG2(c) => Some(c),
            Pixel::BG3(c) => Some(c),
            Pixel::BG4(c) => Some(c),
            Pixel::ObjHi(c) => Some(c),
            Pixel::ObjLo(c) => Some(c),
            Pixel::None => None
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
    fn draw_sprites_to_line(&self, mem: &VideoMem, main_line: &mut [SpritePixel], sub_line: &mut [SpritePixel], y: u8) {
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

        let window_regs = mem.get_window_registers();
        for (x, (main, sub)) in main_line.iter_mut().zip(sub_line.iter_mut()).enumerate() {
            if !window_regs.show_obj_pixel(Screen::Main, x as u8) {
                *main = SpritePixel::Masked;
            }
            if !window_regs.show_obj_pixel(Screen::Sub, x as u8) {
                *sub = SpritePixel::Masked;
            }
        }

        let objects = mem.get_oam();
        
        objects.iter().filter(|object| { // See if this sprite should appear on this line.
            let size_y = if object.large {large.1} else {small.1};

            let bottom_y = object.y.wrapping_add(size_y - 1);
            
            if bottom_y > object.y {
                (y >= object.y) && (y <= bottom_y)
            } else {
                (y >= object.y) || (y <= bottom_y)
            }   // TODO: fix sprite priorities...
        }).take(32).collect::<Vec<_>>().iter().rev().for_each(|object| { // Actually do drawing.
            let size = if object.large {large} else {small};
            let sprite_y = y - object.y;   // TODO: deal with wraparound.
            let y_pixel = if object.y_flip() {size.1 - 1 - sprite_y} else {sprite_y} as usize;

            for x in 0..size.0 {
                let line_x = object.x + x;  // TODO cast to unsigned here.
                if line_x >= 0 && line_x < 256 {  // TODO: no magic number here.
                    if main_line[line_x as usize].is_masked() && sub_line[line_x as usize].is_masked() {
                        continue;
                    }
                    let x_pixel = if object.x_flip() {size.0 - 1 - x} else {x} as usize;
                    let tile_num = object.calc_tile_num(x_pixel, y_pixel);

                    let texel = self.obj_pattern_mem[object.name_table()]
                        .ref_tile(tile_num)
                        .get_texel(x_pixel % 8, y_pixel % 8) as usize;

                    if texel != 0 {
                        let col_index = object.palette_offset() + texel;
                        let colour = self.palettes.get_obj_colour(col_index);
                        let col_math = col_index >= 64; // Top 4 palettes participate in colour math
                        let pix = match object.priority() {
                            SpritePriority::_3 => SpritePixel::Prio3(SpriteColour{colour, col_math}),
                            SpritePriority::_2 => SpritePixel::Prio2(SpriteColour{colour, col_math}),
                            SpritePriority::_1 => SpritePixel::Prio1(SpriteColour{colour, col_math}),
                            SpritePriority::_0 => SpritePixel::Prio0(SpriteColour{colour, col_math}),
                        };
                        if !main_line[line_x as usize].is_masked() {
                            main_line[line_x as usize] = pix;
                        }
                        if !sub_line[line_x as usize].is_masked() {
                            sub_line[line_x as usize] = pix;
                        }
                    }
                }
            }   // for sprite x pixels
        });
    }

    fn draw_bg_to_line(&self, mem: &VideoMem, bg: usize, main_line: &mut [BGData], sub_line: &mut [BGData], y: usize) {
        let regs = mem.get_bg_registers();
        let window_regs = mem.get_window_registers();
        // TODO: separate mosaic stuff?
        let mosaic_amount = if regs.bg_mosaic_enabled(bg) {
            regs.bg_mosaic_mask()
        } else {
            0
        } as usize;
        let mut x_mosaic_offset = 0;

        let y_mosaic_offset = y % (mosaic_amount + 1);
        let bg_y = (y + (regs.get_bg_scroll_y(bg) as usize) - y_mosaic_offset) & self.bg_cache[bg].mask_y();
        let bg_row = self.bg_cache[bg].ref_row(bg_y);

        let x_offset = regs.get_bg_scroll_x(bg) as usize;
        let mask_x = self.bg_cache[bg].mask_x();

        for (x, (main, sub)) in main_line.iter_mut().zip(sub_line.iter_mut()).enumerate() {
            let bg_x = (x + x_offset - x_mosaic_offset) & mask_x;
            if x_mosaic_offset == mosaic_amount {
                x_mosaic_offset = 0;
            } else {
                x_mosaic_offset += 1;
            }
            if window_regs.show_bg_pixel(bg, Screen::Main, x as u8) {
                *main = bg_row[bg_x];
            }
            if window_regs.show_bg_pixel(bg, Screen::Sub, x as u8) {
                *sub = bg_row[bg_x];
            }
        }
    }

    // Make a pixel from a texel and attributes
    #[inline]
    fn make_2bpp_pixel(&self, data: BGData) -> Colour {
        let palette_num = (data.attrs & TileAttributes::PALETTE).bits();
        self.palettes.get_bg_colour((palette_num + data.texel) as usize)
    }
    #[inline]
    fn make_4bpp_pixel(&self, data: BGData) -> Colour {
        let palette_num = (data.attrs & TileAttributes::PALETTE).bits() << 2;
        self.palettes.get_bg_colour((palette_num + data.texel) as usize)
    }

    fn eval_mode_0(&self, sprite_pix: SpritePixel, bg1: BGData, bg2: BGData, bg3: BGData, bg4: BGData) -> Pixel {
        if let SpritePixel::Prio3(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg1.texel != 0 && bg1.attrs.contains(TileAttributes::PRIORITY) {
            Pixel::BG1(self.make_2bpp_pixel(bg1))
        } else if bg2.texel != 0 && bg2.attrs.contains(TileAttributes::PRIORITY) {
            Pixel::BG2(self.make_2bpp_pixel(bg2))
        } else if let SpritePixel::Prio2(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg1.texel != 0 {
            Pixel::BG1(self.make_2bpp_pixel(bg1))
        } else if bg2.texel != 0 {
            Pixel::BG2(self.make_2bpp_pixel(bg2))
        } else if let SpritePixel::Prio1(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg3.texel != 0 && bg3.attrs.contains(TileAttributes::PRIORITY) {
            Pixel::BG3(self.make_2bpp_pixel(bg3))
        } else if bg4.texel != 0 && bg4.attrs.contains(TileAttributes::PRIORITY) {
            Pixel::BG4(self.make_2bpp_pixel(bg4))
        } else if let SpritePixel::Prio0(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg3.texel != 0 {
            Pixel::BG3(self.make_2bpp_pixel(bg3))
        } else if bg4.texel != 0 {
            Pixel::BG4(self.make_2bpp_pixel(bg4))
        } else {
            Pixel::None
        }
    }

    fn eval_mode_1(&self, bg3_hi: bool, sprite_pix: SpritePixel, bg1: BGData, bg2: BGData, bg3: BGData) -> Pixel {
        if bg3_hi && bg3.texel != 0 {
            Pixel::BG3(self.make_2bpp_pixel(bg3))
        } else if let SpritePixel::Prio3(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg1.texel != 0 && bg1.attrs.contains(TileAttributes::PRIORITY) {
            Pixel::BG1(self.make_4bpp_pixel(bg1))
        } else if bg2.texel != 0 && bg2.attrs.contains(TileAttributes::PRIORITY) {
            Pixel::BG2(self.make_4bpp_pixel(bg2))
        } else if let SpritePixel::Prio2(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg1.texel != 0 {
            Pixel::BG1(self.make_4bpp_pixel(bg1))
        } else if bg2.texel != 0 {
            Pixel::BG2(self.make_4bpp_pixel(bg2))
        } else if let SpritePixel::Prio1(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg3.texel != 0 && bg3.attrs.contains(TileAttributes::PRIORITY) {
            Pixel::BG3(self.make_2bpp_pixel(bg3))
        } else if let SpritePixel::Prio0(_) = sprite_pix {
            sprite_pix.pixel()
        } else if bg3.texel != 0 {
            Pixel::BG3(self.make_2bpp_pixel(bg3))
        } else {
            Pixel::None
        }
    }
}
