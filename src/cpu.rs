// SNES Processor
use bitflags::bitflags;

use crate::mem::{
    MemBus,
    MemDevice
};

bitflags! {
    // Flags for status bits inside the CPU.
    #[derive(Default)]
    struct PFlags: u8 {
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

// 65816
pub struct CPU {
    // Registers
    a:      u16,    // Accumulator
    x:      u16,    // X-Index
    y:      u16,    // Y-Index
    s:      u16,    // Stack Pointer
    db:     u8,     // Data Bank
    dp:     u16,    // Direct Page
    pb:     u8,     // Program Bank
    p:      PFlags, // Processor Status
    pc:     u16,    // Program Counter

    // Memory
    mem:    MemBus
}

// Public
impl CPU {
    // Create and initialise new CPU.
    pub fn new() -> Self {
        CPU {
            a:  0,
            x:  0,
            y:  0,
            s:  0,
            db: 0,
            dp: 0,
            pb: 0,
            p:  PFlags::default(),
            pc: 0,

            mem: MemBus::new()
        }
    }
}

// Internal: instructions
impl CPU {
    // Execute a single instruction.
    fn execute_instruction(&mut self) {

    }
}

// Internal: Micro-ops
impl CPU {
    // Fetch a byte from the PC.
    fn fetch(&mut self) -> u8 {
        // Read mem
        let data = self.mem.read(make24!(self.pb, self.pc));
        self.pc = self.pc.wrapping_add(1);
        data
    }
}