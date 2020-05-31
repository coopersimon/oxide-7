// Various constants

// Screen parameters
pub mod screen {
    pub const HORIZONTAL_DOTS: usize = 341;
    pub const NUM_SCANLINES: usize = 262;

    pub const H_RES: usize = 256;
    pub const V_RES: usize = 224;
}

// Master cycle counts
pub mod timing {
    use super::screen;

    // Instruction timing
    pub const INTERNAL_OP: usize      = 6;
    pub const FAST_MEM_ACCESS: usize  = 6;
    pub const SLOW_MEM_ACCESS: usize  = 8;
    pub const XSLOW_MEM_ACCESS: usize = 12;

    // Video timing
    pub const DOT_TIME: usize = 4;
    pub const SCANLINE: usize = DOT_TIME * screen::HORIZONTAL_DOTS;
    pub const SCANLINE_OFFSET: usize = DOT_TIME * 22;
    pub const H_BLANK_TIME: usize = SCANLINE_OFFSET + (DOT_TIME * screen::H_RES);

    // CPU pause in middle of line.
    pub const PAUSE_LEN: usize = 40;
    pub const PAUSE_START: usize = 536;

    // Clock rate of CPU.
    pub const MASTER_HZ: usize = SCANLINE * screen::NUM_SCANLINES * 60;
}

// Interrupt vector locations. Each contains a 16-bit address.
pub mod int {
    pub const COP_VECTOR: u32   = 0xFFE4;
    pub const BRK_VECTOR: u32   = 0xFFE6;
    //pub const ABORT_VECTOR: u32 = 0xFFE8;
    pub const NMI_VECTOR: u32   = 0xFFEA;
    pub const RESET_VECTOR: u32 = 0xFFEC;
    pub const IRQ_VECTOR: u32   = 0xFFEE;

    pub const COP_VECTOR_EMU: u32   = 0xFFF4;
    //pub const ABORT_VECTOR_EMU: u32 = 0xFFF8;
    pub const NMI_VECTOR_EMU: u32   = 0xFFFA;
    pub const RESET_VECTOR_EMU: u32 = 0xFFFC;
    pub const BRK_VECTOR_EMU: u32   = 0xFFFE;
    pub const IRQ_VECTOR_EMU: u32   = 0xFFFE;
}