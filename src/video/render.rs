// Tools for communicating with the renderer (and its thread on certain platforms9.

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
        match val & (bit!(2) | bit!(1) | bit!(0)) {
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