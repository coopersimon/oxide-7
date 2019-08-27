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
        const E = bit!(0);  // 6502 Emulator Mode
    }
}

// Addressing modes
#[derive(Clone, Copy)]
enum AddrMode {
    DirX,
    DirY
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
    pe:     PFlags, // 6502 Emulator Processor Status
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
            pe: PFlags::default(),
            pc: 0,

            mem: MemBus::new()
        }
    }
}

// Internal: High-level
impl CPU {
    // Execute a single instruction.
    fn execute_instruction(&mut self) {
        let instr = self.fetch();

        match instr {
            0x61 => self.adc(AddrMode::DirX),
            0x71 => self.adc(AddrMode::DirY),
            _ => unreachable!()
        }
    }
}

// Internal: Instructions
impl CPU {
    fn adc(&mut self, addr_mode: AddrMode) {
        let op = self.get_op(addr_mode);
        let result = self.a.wrapping_add(op).wrapping_add((self.p & PFlags::C).bits() as u16);

        self.a = if self.p.contains(PFlags::M) {
            let result8 = result & 0xFF;
            let full_wraparound = (result8 == self.a) && (op != 0);
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::V, ((result8 as i8) < (self.a as i8)) || full_wraparound);
            self.p.set(PFlags::Z, result8 == 0);
            self.p.set(PFlags::C, (result8 < self.a) || full_wraparound);

            result8
        } else {
            let full_wraparound = (result == self.a) && (op != 0);
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::V, ((result as i16) < (self.a as i16)) || full_wraparound);
            self.p.set(PFlags::Z, result == 0);
            self.p.set(PFlags::C, (result < self.a) || full_wraparound);

            result
        };
    }
}

// Internal: Memory and Addressing Micro-ops
impl CPU {
    // Read a byte from the (data) bus.
    fn read_data(&self, addr: u32) -> u8 {
        self.mem.read(addr)
    }

    // Fetch a byte from the PC.
    fn fetch(&mut self) -> u8 {
        // Read mem
        let data = self.read_data(make24!(self.pb, self.pc));
        self.pc = self.pc.wrapping_add(1);
        data
    }

    // Get an operand using the specified addressing mode.
    fn get_op(&mut self, addr_mode: AddrMode) -> u16 {
        use self::AddrMode::*;

        match addr_mode {
            DirX => self.dirX(),
            DirY => self.dirY()
        }
    }

    // Addressing modes:
    // DIRECT, X
    fn dirX(&mut self) -> u16 {
        let imm = self.fetch() as u16;

        let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            let addr_lo = (self.x + imm) as u8; // TODO: make a macro for this if it appears a lot.
            set_hi!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.x).wrapping_add(imm)
        };

        let addr_lo = make24!(0, addr);
        let data_lo = self.mem.read(addr_lo);

        let data_hi = if !self.pe.contains(PFlags::M) {
            let addr_hi = make24!(0, addr.wrapping_add(1));
            self.mem.read(addr_hi)
        } else { 0 };
        
        make16!(data_hi, data_lo)
    }

    // DIRECT, Y
    fn dirY(&mut self) -> u16 {
        let imm = self.fetch() as u16;

        let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            let addr_lo = (self.y + imm) as u8; // TODO: make a macro for this if it appears a lot.
            set_hi!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.y.wrapping_add(imm))
        };

        let addr_lo = make24!(0, addr);
        let data_lo = self.read_data(addr_lo);

        let data_hi = if !self.pe.contains(PFlags::M) {
            let addr_hi = make24!(0, addr.wrapping_add(1));
            self.read_data(addr_hi)
        } else { 0 };
        
        make16!(data_hi, data_lo)
    }
}