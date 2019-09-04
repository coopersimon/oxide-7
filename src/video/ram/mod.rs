// Video memory. Connected to the B bus.

mod registers;
mod cgram;
mod oam;
mod vram;

pub use registers::*;
use cgram::CGRAM;
use oam::OAM;
use vram::VRAM;

// Struct containing OAM, CGRAM and VRAM.
pub struct VideoMem {
    registers:      Registers,

    oam:            OAM,
    cgram:          CGRAM,
    vram:           VRAM
}

impl VideoMem {
    pub fn new() -> Self {
        VideoMem {
            registers:  Registers::new(),

            oam:        OAM::new(),
            cgram:      CGRAM::new(),
            vram:       VRAM::new()
        }
    }

    pub fn read(&mut self, addr: u8) -> u8 {
        match addr {
            0x34..=0x37 => 0, // Mult result
            0x38 => self.oam.read(),
            0x39 => self.vram.read_lo(),
            0x3A => self.vram.read_hi(),
            0x3B => self.cgram.read(),
            0x3C => 0, // H scanline pos
            0x3D => 0, // V scanline pos
            0x3E..=0x3F => 0, // PPU status
            _ => unreachable!()
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x00 => self.registers.set_screen_display(data),

            0x01 => self.registers.set_object_settings(data),
            0x02 => self.oam.set_addr_lo(data),
            0x03 => self.oam.set_addr_hi(data),
            0x04 => self.oam.write(data),

            0x05 => self.registers.set_bg_mode(data),
            0x06 => self.registers.set_mosaic(data),
            0x07 => self.registers.bg1_settings = data,
            0x08 => self.registers.bg2_settings = data,
            0x09 => self.registers.bg3_settings = data,
            0x0A => self.registers.bg4_settings = data,
            0x0B => self.registers.bg12_char_addr = data,
            0x0C => self.registers.bg34_char_addr = data,
            0x0D => self.registers.bg1_scroll_x = data,
            0x0E => self.registers.bg1_scroll_y = data,
            0x0F => self.registers.bg2_scroll_x = data,
            0x10 => self.registers.bg2_scroll_y = data,
            0x11 => self.registers.bg3_scroll_x = data,
            0x12 => self.registers.bg3_scroll_y = data,
            0x13 => self.registers.bg4_scroll_x = data,
            0x14 => self.registers.bg4_scroll_y = data,

            0x15 => self.vram.set_port_control(data),
            0x16 => self.vram.set_addr_lo(data),
            0x17 => self.vram.set_addr_hi(data),
            0x18 => self.vram.write_lo(data),
            0x19 => self.vram.write_hi(data),

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

    // Renderer methods to get raw data.
    pub fn get_oam<'a>(&'a mut self) -> (&'a [u8], &'a [u8]) {
        self.oam.ref_data()
    }

    pub fn get_cgram<'a>(&'a mut self) -> &'a [u8] {
        self.cgram.ref_data()
    }

    pub fn get_vram<'a>(&'a self) -> &'a [u8] {
        self.vram.ref_data()
    }

    pub fn get_registers<'a>(&'a self) -> &'a Registers {
        &self.registers
    }

    // Renderer methods to check dirtiness of data.
    pub fn is_oam_dirty(&self) -> bool {
        self.oam.is_dirty()
    }

    pub fn is_cgram_dirty(&self) -> bool {
        self.cgram.is_dirty()
    }

    pub fn is_vram_dirty(&self) -> bool {
        self.vram.is_dirty()
    }

    pub fn vram_dirty_range(&self, start: u16, end: u16) -> bool {
        self.vram.dirty_range(start, end)
    }

    pub fn vram_reset_dirty_range(&mut self) {
        self.vram.reset_dirty_range();
    }
}