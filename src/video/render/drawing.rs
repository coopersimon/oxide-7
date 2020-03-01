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

// Drawing types
#[derive(Clone, Copy)]
enum SpritePixel {
    Prio3(Colour),
    Prio2(Colour),
    Prio1(Colour),
    Prio0(Colour),
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
}

enum Pixel {
    BG1(Colour),
    BG2(Colour),
    BG3(Colour),
    BG4(Colour),
    Obj(Colour),
    None
}

impl Pixel {
    fn any(self) -> Option<Colour> {
        match self {
            Pixel::BG1(c) => Some(c),
            Pixel::BG2(c) => Some(c),
            Pixel::BG3(c) => Some(c),
            Pixel::BG4(c) => Some(c),
            Pixel::Obj(c) => Some(c),
            Pixel::None => None
        }
    }
}

// Struct for lazy-loading metadata about background pixels, and evaluating priorities.
struct PixelData {
    bg1: Option<BGData>,
    bg2: Option<BGData>,
    bg3: Option<BGData>,
    bg4: Option<BGData>,
    main_obj: SpritePixel,
    sub_obj: SpritePixel,
}

impl PixelData {
    fn new(main_obj: SpritePixel, sub_obj: SpritePixel) -> Self {
        PixelData {
            bg1: None,
            bg2: None,
            bg3: None,
            bg4: None,
            main_obj: main_obj,
            sub_obj: sub_obj,
        }
    }

    #[inline]
    fn get_obj(&self, screen: Screen) -> SpritePixel {
        match screen {
            Screen::Main => self.main_obj,
            Screen::Sub => self.sub_obj
        }
    }

    #[inline]
    fn get_bg1(&mut self, renderer: &Renderer, regs: &Registers, x: usize, y: usize) -> BGData {
        if let Some(data) = self.bg1 {
            data
        } else {
            let data = renderer.get_bg_data(regs, 0, x, y);
            self.bg1 = Some(data);
            data
        }
    }
    #[inline]
    fn get_bg2(&mut self, renderer: &Renderer, regs: &Registers, x: usize, y: usize) -> BGData {
        if let Some(data) = self.bg2 {
            data
        } else {
            let data = renderer.get_bg_data(regs, 1, x, y);
            self.bg2 = Some(data);
            data
        }
    }
    #[inline]
    fn get_bg3(&mut self, renderer: &Renderer, regs: &Registers, x: usize, y: usize) -> BGData {
        if let Some(data) = self.bg3 {
            data
        } else {
            let data = renderer.get_bg_data(regs, 2, x, y);
            self.bg3 = Some(data);
            data
        }
    }
    #[inline]
    fn get_bg4(&mut self, renderer: &Renderer, regs: &Registers, x: usize, y: usize) -> BGData {
        if let Some(data) = self.bg4 {
            data
        } else {
            let data = renderer.get_bg_data(regs, 3, x, y);
            self.bg4 = Some(data);
            data
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
                        let colour = self.palettes.get_obj_colour(object.palette_offset() + texel);
                        let pix = match object.priority() {
                            SpritePriority::_3 => SpritePixel::Prio3(colour),
                            SpritePriority::_2 => SpritePixel::Prio2(colour),
                            SpritePriority::_1 => SpritePixel::Prio1(colour),
                            SpritePriority::_0 => SpritePixel::Prio0(colour),
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

    // Get a texel and attribute pair for a BG pixel.
    fn get_bg_data(&self, regs: &Registers, bg: usize, x: usize, y: usize) -> BGData {
        let bg_x = (x + (regs.get_bg_scroll_x(bg) as usize)) & self.bg_cache[bg].mask_x();   // TODO: replace with &
        let bg_y = (y + (regs.get_bg_scroll_y(bg) as usize)) & self.bg_cache[bg].mask_y();

        if regs.bg_mosaic_enabled(bg) {
            let mosaic_mask = regs.bg_mosaic_mask() as usize;
            self.bg_cache[bg].get_data((bg_x / mosaic_mask) * mosaic_mask, (bg_y / mosaic_mask) * mosaic_mask)
        } else {
            self.bg_cache[bg].get_data(bg_x, bg_y)
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

    // Returns a pixel.
    fn eval_mode_0(&self, mem: &VideoMem, mut pixels: PixelData, x: usize, y: usize) -> (Pixel, Colour) {
        let window_regs = mem.get_window_registers();

        let main = self.eval_mode_0_screen(mem, Screen::Main, &mut pixels, x, y);
        let sub = if window_regs.use_subscreen() {
            self.eval_mode_0_screen(mem, Screen::Sub, &mut pixels, x, y).any()
                .unwrap_or(window_regs.get_fixed_colour())
        } else {
            window_regs.get_fixed_colour()
        };

        (main, sub)
    }

    fn eval_mode_0_screen(&self, mem: &VideoMem, screen: Screen, pixels: &mut PixelData, x: usize, y: usize) -> Pixel {
        let bg_regs = mem.get_bg_registers();
        let window_regs = mem.get_window_registers();

        if let SpritePixel::Prio3(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        let show_bg1 = window_regs.show_bg_pixel(0, screen, x as u8);
        if show_bg1 {
            let bg1 = pixels.get_bg1(self, bg_regs, x, y);
            if bg1.texel != 0 && bg1.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG1(self.make_2bpp_pixel(bg1));
            }
        }
        let show_bg2 = window_regs.show_bg_pixel(1, screen, x as u8);
        if show_bg2 {
            let bg2 = pixels.get_bg2(self, bg_regs, x, y);
            if bg2.texel != 0 && bg2.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG2(self.make_2bpp_pixel(bg2));
            }
        }
        if let SpritePixel::Prio2(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        if show_bg1 {
            let bg1 = pixels.get_bg1(self, bg_regs, x, y);
            if bg1.texel != 0 {
                return Pixel::BG1(self.make_2bpp_pixel(bg1));
            }
        }
        if show_bg2 {
            let bg2 = pixels.get_bg2(self, bg_regs, x, y);
            if bg2.texel != 0 {
                return Pixel::BG2(self.make_2bpp_pixel(bg2));
            }
        }
        if let SpritePixel::Prio1(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        let show_bg3 = window_regs.show_bg_pixel(2, screen, x as u8);
        if show_bg3 {
            let bg3 = pixels.get_bg3(self, bg_regs, x, y);
            if bg3.texel != 0 && bg3.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG3(self.make_2bpp_pixel(bg3));
            }
        }
        let show_bg4 = window_regs.show_bg_pixel(3, screen, x as u8);
        if show_bg4 {
            let bg4 = pixels.get_bg4(self, bg_regs, x, y);
            if bg4.texel != 0 && bg4.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG4(self.make_2bpp_pixel(bg4));
            }
        }
        if let SpritePixel::Prio0(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        if show_bg3 {
            let bg3 = pixels.get_bg3(self, bg_regs, x, y);
            if bg3.texel != 0 {
                return Pixel::BG3(self.make_2bpp_pixel(bg3));
            }
        }
        if show_bg4 {
            let bg4 = pixels.get_bg4(self, bg_regs, x, y);
            if bg4.texel != 0 {
                return Pixel::BG4(self.make_2bpp_pixel(bg4));
            }
        }

        Pixel::None
    }

    // Returns a pixel.
    fn eval_mode_1(&self, mem: &VideoMem, mut pixels: PixelData, x: usize, y: usize) -> (Pixel, Colour) {
        let window_regs = mem.get_window_registers();

        let main = self.eval_mode_1_screen(mem, Screen::Main, &mut pixels, x, y);
        let sub = if window_regs.use_subscreen() {
            self.eval_mode_1_screen(mem, Screen::Sub, &mut pixels, x, y).any()
                .unwrap_or(window_regs.get_fixed_colour())
        } else {
            window_regs.get_fixed_colour()
        };

        (main, sub)
    }

    fn eval_mode_1_screen(&self, mem: &VideoMem, screen: Screen, pixels: &mut PixelData, x: usize, y: usize) -> Pixel {
        let bg_regs = mem.get_bg_registers();
        let window_regs = mem.get_window_registers();

        let show_bg3 = window_regs.show_bg_pixel(2, screen, x as u8);
        if show_bg3 && bg_regs.get_bg3_priority() {
            let bg3 = pixels.get_bg3(self, bg_regs, x, y);
            if bg3.texel != 0 && bg3.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG3(self.make_2bpp_pixel(bg3));
            }
        }
        if let SpritePixel::Prio3(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        let show_bg1 = window_regs.show_bg_pixel(0, screen, x as u8);
        if show_bg1 {
            let bg1 = pixels.get_bg1(self, bg_regs, x, y);
            if bg1.texel != 0 && bg1.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG1(self.make_4bpp_pixel(bg1));
            }
        }
        let show_bg2 = window_regs.show_bg_pixel(1, screen, x as u8);
        if show_bg2 {
            let bg2 = pixels.get_bg2(self, bg_regs, x, y);
            if bg2.texel != 0 && bg2.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG2(self.make_4bpp_pixel(bg2));
            }
        }
        if let SpritePixel::Prio2(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        if show_bg1 {
            let bg1 = pixels.get_bg1(self, bg_regs, x, y);
            if bg1.texel != 0 {
                return Pixel::BG1(self.make_4bpp_pixel(bg1));
            }
        }
        if show_bg2 {
            let bg2 = pixels.get_bg2(self, bg_regs, x, y);
            if bg2.texel != 0 {
                return Pixel::BG2(self.make_4bpp_pixel(bg2));
            }
        }
        if let SpritePixel::Prio1(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        if show_bg3 && !bg_regs.get_bg3_priority() {
            let bg3 = pixels.get_bg3(self, bg_regs, x, y);
            if bg3.texel != 0 && bg3.attrs.contains(TileAttributes::PRIORITY) {
                return Pixel::BG3(self.make_2bpp_pixel(bg3));
            }
        }
        if let SpritePixel::Prio0(c) = pixels.get_obj(screen) {
            return Pixel::Obj(c);
        }
        if show_bg3 {
            let bg3 = pixels.get_bg3(self, bg_regs, x, y);
            if bg3.texel != 0 {
                return Pixel::BG3(self.make_2bpp_pixel(bg3));
            }
        }

        Pixel::None
    }
}

// Drawing modes
impl Renderer {
    fn draw_line_mode_0(&self, mem: &VideoMem, target: &mut [u8], y: usize) {
        let target_start = y * SCREEN_WIDTH;

        let mut main_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        let mut sub_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        self.draw_sprites_to_line(mem, &mut main_sprite_pixels, &mut sub_sprite_pixels, y as u8);

        for (x, i) in target.chunks_mut(4).skip(target_start).take(SCREEN_WIDTH).enumerate() {
            let main_sprite_pix = main_sprite_pixels[x];
            let sub_sprite_pix = sub_sprite_pixels[x];

            let (main, sub) = self.eval_mode_0(mem, PixelData::new(main_sprite_pix, sub_sprite_pix), x, y);

            let col = match main {
                Pixel::BG1(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 0, x as u8),
                Pixel::BG2(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 1, x as u8),
                Pixel::BG3(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 2, x as u8),
                Pixel::BG4(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 3, x as u8),
                Pixel::Obj(c) => mem.get_window_registers().calc_colour_math_obj(c, sub, x as u8),
                Pixel::None => mem.get_window_registers().calc_colour_math_backdrop(self.palettes.get_bg_colour(0), sub, x as u8),
            };

            write_pixel(i, col);
        }
    }

    fn draw_line_mode_1(&self, mem: &VideoMem, target: &mut [u8], y: usize) {
        let target_start = y * SCREEN_WIDTH;

        let mut main_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        let mut sub_sprite_pixels = [SpritePixel::None; SCREEN_WIDTH];
        self.draw_sprites_to_line(mem, &mut main_sprite_pixels, &mut sub_sprite_pixels, y as u8);

        for (x, i) in target.chunks_mut(4).skip(target_start).take(SCREEN_WIDTH).enumerate() {
            let main_sprite_pix = main_sprite_pixels[x];
            let sub_sprite_pix = sub_sprite_pixels[x];

            let (main, sub) = self.eval_mode_1(mem, PixelData::new(main_sprite_pix, sub_sprite_pix), x, y);

            let col = match main {
                Pixel::BG1(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 0, x as u8),
                Pixel::BG2(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 1, x as u8),
                Pixel::BG3(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 2, x as u8),
                Pixel::BG4(c) => mem.get_window_registers().calc_colour_math_bg(c, sub, 3, x as u8),
                Pixel::Obj(c) => mem.get_window_registers().calc_colour_math_obj(c, sub, x as u8),
                Pixel::None => mem.get_window_registers().calc_colour_math_backdrop(self.palettes.get_bg_colour(0), sub, x as u8),
            };

            write_pixel(i, col);
        }
    }
}