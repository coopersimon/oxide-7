// Tools for communicating with the renderer (and its thread on certain platforms).

//mod bgcache;
mod patternmem;
mod drawing;
mod palette;

use std::sync::{
    Arc, Mutex
};

// Renderer trait.
pub trait Renderable {
    fn frame_start(&mut self);
    fn draw_line(&mut self, y: u16);
    fn frame_end(&mut self);
}

// Mode
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VideoMode {
    _0 = 0,
    _1 = 1,
    _2 = 2,
    _3 = 3,
    _4 = 4,
    _5 = 5,
    _6 = 6,
    _7 = 7
}

impl From<u8> for VideoMode {
    fn from(val: u8) -> Self {
        match val & bits![2, 1, 0] {
            0 => VideoMode::_0,
            1 => VideoMode::_1,
            2 => VideoMode::_2,
            3 => VideoMode::_3,
            4 => VideoMode::_4,
            5 => VideoMode::_5,
            6 => VideoMode::_6,
            7 => VideoMode::_7,
            _ => unreachable!()
        }
    }
}

pub type RenderTarget = Arc<Mutex<Box<[u8]>>>;

#[derive(Clone, Copy, Debug)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Colour {
            r: r,
            g: g,
            b: b
        }
    }

    pub const fn zero() -> Colour {
        Colour {
            r: 0,
            g: 0,
            b: 0
        }
    }

    /*const fn red() -> Colour {
        Colour {
            r: 255,
            g: 0,
            b: 0
        }
    }
    const fn blue() -> Colour {
        Colour {
            r: 0,
            g: 0,
            b: 255
        }
    }
    const fn green() -> Colour {
        Colour {
            r: 0,
            g: 255,
            b: 0
        }
    }
    const fn yellow() -> Colour {
        Colour {
            r: 255,
            g: 255,
            b: 0
        }
    }*/
}

/// Reads from video memory and renders a bitmap to a target.
pub struct Renderer {
    target:     Option<RenderTarget>,
    renderer:   drawing::Renderer,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            target:     None,
            renderer:   drawing::Renderer::new(),
        }
    }

    pub fn start_frame(&mut self, target: RenderTarget) {
        self.target = Some(target);
    }

    pub fn draw_line(&mut self, mem: &mut super::VideoMem, y: usize) {
        if !mem.get_bg_registers().in_fblank() {
            self.renderer.setup_caches(mem);
            let mut t = self.target.as_ref().unwrap().lock().unwrap();
            self.renderer.draw_line(&mem, &mut t, y);
        } else {
            let mut t = self.target.as_ref().unwrap().lock().unwrap();
            clear_line(&mut t, y);
        }
    }
}

fn clear_line(target: &mut [u8], y: usize) {
    use crate::constants::screen::H_RES;

    for d in target.iter_mut().skip(y * H_RES * 8).take(H_RES * 8) {
        *d = 0;
    }
}