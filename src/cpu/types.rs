// Types used inside the CPU.

use bitflags::bitflags;


bitflags! {
    // Flags for status bits inside the CPU.
    #[derive(Default)]
    pub struct PFlags: u8 {
        const N = bit!(7);  // Negative
        const V = bit!(6);  // Overflow
        const M = bit!(5);  // Accumulator reg size
        const X = bit!(4);  // Index reg size
        const D = bit!(3);  // Decimal
        const I = bit!(2);  // IRQ disable
        const Z = bit!(1);  // Zero
        const C = bit!(0);  // Carry

        const B = bit!(5);  // Break
    }
}

// Address types.
#[derive(Clone, Copy)]
pub enum Addr {
    Full(u32),      // A full, wrapping, 24-bit address.
    ZeroBank(u16)   // A 16-bit address that wraps at bank boundaries.
}

// Data modes.
#[derive(Clone, Copy)]
pub enum DataMode {
    Imm,                // Immediate data after the instruction
    Acc,                // Accumulator data
    Mode(DataAddrMode), // Find the address using the given Addressing mode
    Known(Addr)         // Use the address provided
}

// Addressing modes for data.
#[derive(Clone, Copy)]
pub enum DataAddrMode {
    Abs,
    AbsX,
    AbsY,

    Dir,
    DirX,
    DirY,
    DirPtrDbr,
    DirPtrXDbr,
    DirPtrDbrY,
    DirPtr,
    DirPtrY,

    Long,
    LongX,
    Stack,
    StackPtrDbrY
}

// Addressing modes for branches and jumps.
#[derive(Clone, Copy, PartialEq)]
pub enum ProgramAddrMode {
    Abs,
    AbsPtrPbr,
    AbsPtrXPbr,
    AbsPtr,
    Long
}