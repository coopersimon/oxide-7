// Tools for communicating with the renderer and its thread.
use crate::joypad::Button;

// Commands to send to the renderer.
pub enum VideoCommand {
    FrameStart,     // Start the process of rendering a frame.
    DrawLine(u8),   // Draw a single line.
    FrameEnd,       // End the process of rendering a frame.
    None            // Null command, just return none.
}

// Signals sent back from the renderer.
pub enum VideoSignal {
    HBlank,         // The line has been drawn, trigger h-blank
    VBlank(Button), // The frame has been rendered fully, trigger v-blank. Return joypad update TODO: make this cleaner
    None            // Null signal, take no action.
}

// Renderer trait.
pub trait Renderable {
    fn frame_start(&mut self);
    fn draw_line(&mut self, y: u8);
    fn frame_end(&mut self);
}

// Mode
#[derive(Clone, Copy, PartialEq)]
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