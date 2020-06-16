// Various constants used internally in the SuperFX chip.

// Special register uses
pub const PLOT_X_REG: usize = 1;
pub const PLOT_Y_REG: usize = 2;
pub const MULT_DST_REG: usize = 4;
pub const MULT_OP_REG: usize = 6;
pub const MERGE_HI_REG: usize = 7;
pub const MERGE_LO_REG: usize = 8;
pub const LINK_REG: usize = 11;
pub const LOOP_CTR_REG: usize = 12;
pub const LOOP_PTR_REG: usize = 13;
pub const ROM_PTR_REG: usize = 14;
pub const PC_REG: usize = 15;

// Timing
pub const RAM_WAIT_CYCLES: usize = 4;
pub const ROM_WAIT_CYCLES: usize = 3;