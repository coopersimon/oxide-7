// Tools for communicating with the renderer and its thread.
use crate::joypad::Button;

// Commands to send to the renderer.
pub enum VideoCommand {
    FrameStart,     // Start the process of rendering a frame.
    DrawLine,       // Draw a single line.
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
    fn draw_line(&mut self);
    fn frame_end(&mut self);
}