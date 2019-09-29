// Types and constants used internally in the SPC-700.

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct PSFlags: u8 {
        const N = bit!(7);  // Negative
        const V = bit!(6);  // Overflow
        const P = bit!(5);  // Direct page
        const B = bit!(4);  // Break
        const H = bit!(3);  // Half carry
        const I = bit!(2);  // Interrupt
        const Z = bit!(1);  // Zero
        const C = bit!(0);  // Carry
    }
}

pub enum DataMode {
    Imm,            // Immediate data
    Acc,            // Accumulator register data
    X,              // X register data
    Y,              // Y register data
    Mode(AddrMode), // Data in memory
    Known(u16)      // Data in memory with known address
}

pub enum AddrMode {
    XIndir,     // (X)
    YIndir,     // (Y)
    XIndirInc,  // (X)+

    Dir,        // (d)
    DirX,       // (d+X)
    DirY,       // (d+Y)
    DirPtrY,    // [d]+Y
    DirXPtr,    // [d+X]

    Abs,        // !a
    AbsX,       // !a+X
    AbsY,       // !a+Y
}

pub const SPC_OP: usize     = 1;        // Number of cycles for an internal op.
pub const STACK_PAGE: u16   = 0x0100;   // Page used for the stack.
pub const U_PAGE: u16       = 0xFF00;   // Page used for pcall.