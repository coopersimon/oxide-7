// Video memory. Connected to the B bus.

mod cgram;

use cgram::CGRAM;

// Struct containing OAM, CGRAM and VRAM.
pub struct VideoMem {
    // OAM
    cgram: CGRAM,
    // VRAM
}

impl VideoMem {
    pub fn new() -> Self {
        VideoMem {
            cgram: CGRAM::new()
        }
    }

    pub fn read(&mut self, addr: u8) -> u8 {
        match addr {
            0x34..=0x37 => 0, // Mult result
            0x38 => 0, // OAM read
            0x39..=0x3A => 0, // VRAM read
            0x3B => self.cgram.read(),
            0x3C => 0, // H scanline pos
            0x3D => 0, // V scanline pos
            0x3E..=0x3F => 0, // PPU status
            _ => unreachable!()
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x00 => {}, // screen display reg
            0x01 => {}, // object control
            0x02 => {}, // OAM address
            0x03 => {},
            0x04 => {}, // OAM write
            0x05 => {}, // bg mode and char size
            0x06 => {}, // mosaic settings
            0x07 => {}, // BG1 settings
            0x08 => {}, // BG2 settings
            0x09 => {}, // BG3 settings
            0x0A => {}, // BG4 settings
            0x0B => {}, // BG1&2 char address
            0x0C => {}, // BG3&4 char address
            0x0D => {}, // BG1 scroll X
            0x0E => {}, // BG1 scroll Y
            0x0F => {}, // BG2 scroll X
            0x10 => {}, // BG2 scroll Y
            0x11 => {}, // BG3 scroll X
            0x12 => {}, // BG3 scroll Y
            0x13 => {}, // BG4 scroll X
            0x14 => {}, // BG4 scroll Y
            0x15 => {}, // Video port control
            0x16 => {}, // VRAM addr lo
            0x17 => {}, // VRAM addr hi
            0x18 => {}, // VRAM data lo
            0x19 => {}, // VRAM data hi
            0x1A..=0x20 => {}, // Mode 7 shit
            0x21 => self.cgram.set_addr(data),
            0x22 => self.cgram.write(data),
            0x23 => {}, // BG1&2 window
            0x24 => {}, // BG3&4 window
            0x25 => {}, // Obj window
            0x26..=0x29 => {}, // Window pos regs
            0x2A..=0x2B => {}, // Window logic regs
            0x2C..=0x2D => {}, // Screen dest regs
            0x2E..=0x2F => {}, // Window mask dest regs
            0x30..=0x32 => {}, // Color math regs
            0x33 => {}, // Screen mode select
            _ => unreachable!()
        }
    }
}