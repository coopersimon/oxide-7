// Video memory. Connected to the B bus.

mod bgregs;
mod cgram;
mod oam;
mod vram;
mod windowregs;

pub use bgregs::*;
use cgram::CGRAM;
use oam::OAM;
pub use oam::SpritePriority;
use vram::VRAM;
pub use windowregs::{
    Screen,
    WindowRegisters
};

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
            0x34 => self.bgregs.read_mult_result_lo(),
            0x35 => self.bgregs.read_mult_result_mid(),
            0x36 => self.bgregs.read_mult_result_hi(),
            0x38 => self.oam.read(),
            0x39 => self.vram.read_lo(),
            0x3A => self.vram.read_hi(),
            0x3B => self.cgram.read(),
            0x3C => if !self.h_hi_byte {
                self.h_hi_byte = true;
                lo!(self.h_pos)
            } else {
                self.h_hi_byte = false;
                hi!(self.h_pos)
            },
            0x3D => if !self.v_hi_byte {
                self.v_hi_byte = true;
                lo!(self.v_pos)
            } else {
                self.v_hi_byte = false;
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
            0x07 => self.bgregs.set_bg1_settings(data),
            0x08 => self.bgregs.set_bg2_settings(data),
            0x09 => self.bgregs.set_bg3_settings(data),
            0x0A => self.bgregs.set_bg4_settings(data),
            0x0B => self.bgregs.set_bg12_char_addr(data),
            0x0C => self.bgregs.set_bg34_char_addr(data),
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

            0x1A => self.bgregs.set_mode7_settings(data),
            0x1B => self.bgregs.set_mode7_matrix_a(data),
            0x1C => self.bgregs.set_mode7_matrix_b(data),
            0x1D => self.bgregs.set_mode7_matrix_c(data),
            0x1E => self.bgregs.set_mode7_matrix_d(data),
            0x1F => self.bgregs.set_mode7_centre_x(data),
            0x20 => self.bgregs.set_mode7_centre_y(data),

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
            0x33 => self.windowregs.set_video_select(data),
            _ => unreachable!()
        }
    }

    // Set latched h or v pos.
    pub fn set_latched_hv(&mut self, h: u16, v: u16) {
        self.h_pos = h;
        self.v_pos = v;
    }

    // OAM address reset that happens at V-blank
    pub fn oam_reset(&mut self) {
        self.oam.reset();
    }

    // Renderer methods to get raw data.
    pub fn get_oam<'a>(&'a self) -> &'a [oam::Object] {
        self.oam.ref_data()
    }

    pub fn get_cgram<'a>(&'a self) -> &'a [u8] {
        self.cgram.ref_data()
    }

    pub fn get_vram<'a>(&'a self) -> &'a [u8] {
        self.vram.ref_data()
    }

    pub fn get_bg_registers<'a>(&'a self) -> &'a Registers {
        &self.bgregs
    }

    pub fn get_window_registers<'a>(&'a self) -> &'a WindowRegisters {
        &self.windowregs
    }

    // Renderer methods to check dirtiness of data.
    pub fn is_cgram_bg_dirty(&self) -> bool {
        self.cgram.is_bg_dirty()
    }

    pub fn is_cgram_obj_dirty(&self) -> bool {
        self.cgram.is_obj_dirty()
    }

    pub fn cgram_reset_dirty(&mut self) {
        self.cgram.reset_dirty()
    }

    pub fn vram_is_dirty(&self, start_addr: u16) -> bool {
        self.vram.dirty_range(start_addr)
    }

    pub fn vram_reset_dirty_range(&mut self, read: &[u16]) {
        self.vram.reset_dirty_range(read);
    }

    pub fn vram_set_pattern_regions(&mut self, regions: Vec<(u16, u16)>) {
        self.vram.set_pattern_regions(regions);
    }
}