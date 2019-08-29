// Various constants

// Master cycle counts
pub mod timing {
    pub const INTERNAL_OP: usize      = 6;
    pub const FAST_MEM_ACCESS: usize  = 6;
    pub const SLOW_MEM_ACCESS: usize  = 8;
    pub const XSLOW_MEM_ACCESS: usize = 12;

    pub const SCANLINE: usize = 1364;
    pub const PAUSE_START: usize = 536;
    pub const PAUSE_LEN: usize = 40;

    pub const NUM_SCANLINES: usize = 262;
}

// Interrupt vector locations. Each contains a 16-bit address.
pub mod int {
    pub const COP_VECTOR: u32   = 0xFFE4;
    pub const BRK_VECTOR: u32   = 0xFFE6;
    pub const ABORT_VECTOR: u32 = 0xFFE8;
    pub const NMI_VECTOR: u32   = 0xFFEA;
    pub const RESET_VECTOR: u32 = 0xFFEC;
    pub const IRQ_VECTOR: u32   = 0xFFEE;

    pub const COP_VECTOR_EMU: u32   = 0xFFF4;
    pub const ABORT_VECTOR_EMU: u32 = 0xFFF8;
    pub const NMI_VECTOR_EMU: u32   = 0xFFFA;
    pub const RESET_VECTOR_EMU: u32 = 0xFFFC;
    pub const BRK_VECTOR_EMU: u32   = 0xFFFE;
    pub const IRQ_VECTOR_EMU: u32   = 0xFFFE;
}