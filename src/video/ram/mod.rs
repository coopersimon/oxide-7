// Video memory. Connected to the B bus.

mod bgregs;
mod cgram;
mod oam;
mod vram;
mod windowregs;

pub use bgregs::*;
use cgram::CGRAM;
use oam::OAM;
use vram::VRAM;
use windowregs::WindowRegisters;

// Struct containing OAM, CGRAM and VRAM.
pub struct VideoMem {
    bgregs:         Registers,
    windowregs:     WindowRegisters,

    h_pos:          u16,
    v_pos:          u16,

    h_hi_byte:      bool,
    v_hi_byte:      bool,

    oam:            OAM,
    cgram:          CGRAM,
    vram:           VRAM
}

impl VideoMem {
    pub fn new() -> Self {
        VideoMem {
            bgregs:     Registers::new(),
            windowregs: WindowRegisters::new(),

            h_pos:      0,
            v_pos:      0,

            h_hi_byte:  false,
            v_hi_byte:  false,

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
            0x3C => if !self.h_hi_byte {
                self.h_hi_byte = true;
                lo!(self.h_pos)
            } else {
                hi!(self.h_pos)
            },
            0x3D => if !self.v_hi_byte {
                self.v_hi_byte = true;
                lo!(self.v_pos)
            } else {
                hi!(self.v_pos)
            },
            0x3E => 1,  // PPU Status
            0x3F => {   // PPU Status
                self.h_hi_byte = false;
                self.v_hi_byte = false;
                2
            },
            _ => unreachable!()
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x00 => self.bgregs.set_screen_display(data),

            0x01 => self.bgregs.set_object_settings(data),
            0x02 => self.oam.set_addr_lo(data),
            0x03 => self.oam.set_addr_hi(data),
            0x04 => self.oam.write(data),

            0x05 => self.bgregs.set_bg_mode(data),
            0x06 => self.bgregs.set_mosaic(data),
            0x07 => self.bgregs.bg1_settings = data,
            0x08 => self.bgregs.bg2_settings = data,
            0x09 => self.bgregs.bg3_settings = data,
            0x0A => self.bgregs.bg4_settings = data,
            0x0B => self.bgregs.bg12_char_addr = data,
            0x0C => self.bgregs.bg34_char_addr = data,
            0x0D => self.bgregs.set_bg1_scroll_x(data),
            0x0E => self.bgregs.set_bg1_scroll_y(data),
            0x0F => self.bgregs.set_bg2_scroll_x(data),
            0x10 => self.bgregs.set_bg2_scroll_y(data),
            0x11 => self.bgregs.set_bg3_scroll_x(data),
            0x12 => self.bgregs.set_bg3_scroll_y(data),
            0x13 => self.bgregs.set_bg4_scroll_x(data),
            0x14 => self.bgregs.set_bg4_scroll_y(data),

            0x15 => self.vram.set_port_control(data),
            0x16 => self.vram.set_addr_lo(data),
            0x17 => self.vram.set_addr_hi(data),
            0x18 => self.vram.write_lo(data),
            0x19 => self.vram.write_hi(data),

            0x1A..=0x20 => {}, // Mode 7 shit

            0x21 => self.cgram.set_addr(data),
            0x22 => self.cgram.write(data),
            
            0x23 => self.windowregs.set_mask_bg1_2(data), // BG1&2 window
            0x24 => self.windowregs.set_mask_bg3_4(data), // BG3&4 window
            0x25 => self.windowregs.set_mask_obj_col(data), // Obj window
            0x26 => self.windowregs.window_1_left = data,
            0x27 => self.windowregs.window_1_right = data,
            0x28 => self.windowregs.window_2_left = data,
            0x29 => self.windowregs.window_2_right = data,
            0x2A => self.windowregs.set_mask_logic_bg(data),
            0x2B => self.windowregs.set_mask_logic_obj_col(data),
            0x2C => self.windowregs.set_main_screen_desg(data),
            0x2D => self.windowregs.set_sub_screen_desg(data),
            0x2E => self.windowregs.set_main_mask_desg(data),
            0x2F => self.windowregs.set_sub_mask_desg(data),
            0x30 => self.windowregs.set_colour_add_select(data),
            0x31 => self.windowregs.set_colour_math_desg(data),
            0x32 => self.windowregs.set_fixed_colour(data),
            0x33 => {}, // Screen mode select
            _ => unreachable!()
        }
    }

    // Set latched h or v pos.
    pub fn set_latched_hv(&mut self, h: u16, v: u16) {
        self.h_pos = h;
        self.v_pos = v;
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
        &self.bgregs
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