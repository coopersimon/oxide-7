// SNES Processor
use bitflags::bitflags;

use crate::{
    mem::MemBus,
    timing::INTERNAL_OP
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

// Addresses
#[derive(Clone, Copy)]
enum Addr {
    Full(u32),      // A full, wrapping, 24-bit address.
    ZeroBank(u16)   // A 16-bit address that wraps at bank boundaries.
}

// Data modes
#[derive(Clone, Copy)]
enum DataMode {
    Imm,                // Immediate data after the instruction
    Acc,                // Accumulator data
    Mode(DataAddrMode), // Find the address using the given Addressing mode
    Known(Addr)         // Use the address provided
}

// Addressing modes for data
#[derive(Clone, Copy)]
enum DataAddrMode {
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

// Addressing modes for branches and jumps
#[derive(Clone, Copy, PartialEq)]
enum JumpAddrMode {
    Abs,
    AbsPtrPbr,
    AbsPtrXPbr,
    AbsPtr,
    Long
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

    // Status
    halt:   bool,

    // Memory
    mem:    MemBus
}

// Public
impl CPU {
    // Create and initialise new CPU.
    pub fn new(bus: MemBus) -> Self {
        CPU {
            a:      0,
            x:      0,
            y:      0,
            s:      0,
            db:     0,
            dp:     0,
            pb:     0,
            p:      PFlags::default(),
            pe:     PFlags::default(),
            pc:     0,

            halt:   false,

            mem:    bus
        }
    }

    // A single step of the CPU.
    // Executes an instruction and clocks other components.
    pub fn step(&mut self) {
        if !self.halt {
            self.execute_instruction();
        }
    }
}

// Internal: High-level
impl CPU {
    // Execute a single instruction.
    fn execute_instruction(&mut self) {
        use self::DataAddrMode::*;
        use self::DataMode::*;

        let instr = self.fetch();

        match instr {
            0x61 => self.adc(Mode(DirPtrXDbr)),
            0x63 => self.adc(Mode(Stack)),
            0x65 => self.adc(Mode(Dir)),
            0x67 => self.adc(Mode(DirPtr)),
            0x69 => self.adc(Imm),
            0x6D => self.adc(Mode(Abs)),
            0x6F => self.adc(Mode(Long)),
            0x71 => self.adc(Mode(DirPtrDbrY)),
            0x72 => self.adc(Mode(DirPtrDbr)),
            0x73 => self.adc(Mode(StackPtrDbrY)),
            0x75 => self.adc(Mode(DirX)),
            0x77 => self.adc(Mode(DirPtrY)),
            0x79 => self.adc(Mode(AbsY)),
            0x7D => self.adc(Mode(AbsX)),
            0x7F => self.adc(Mode(LongX)),

            0xE1 => self.sbc(Mode(DirPtrXDbr)),
            0xE3 => self.sbc(Mode(Stack)),
            0xE5 => self.sbc(Mode(Dir)),
            0xE7 => self.sbc(Mode(DirPtr)),
            0xE9 => self.sbc(Imm),
            0xED => self.sbc(Mode(Abs)),
            0xEF => self.sbc(Mode(Long)),
            0xF1 => self.sbc(Mode(DirPtrDbrY)),
            0xF2 => self.sbc(Mode(DirPtrDbr)),
            0xF3 => self.sbc(Mode(StackPtrDbrY)),
            0xF5 => self.sbc(Mode(DirX)),
            0xF7 => self.sbc(Mode(DirPtrY)),
            0xF9 => self.sbc(Mode(AbsY)),
            0xFD => self.sbc(Mode(AbsX)),
            0xFF => self.sbc(Mode(LongX)),

            0xC1 => self.cmp(Mode(DirPtrXDbr)),
            0xC3 => self.cmp(Mode(Stack)),
            0xC5 => self.cmp(Mode(Dir)),
            0xC7 => self.cmp(Mode(DirPtr)),
            0xC9 => self.cmp(Imm),
            0xCD => self.cmp(Mode(Abs)),
            0xCF => self.cmp(Mode(Long)),
            0xD1 => self.cmp(Mode(DirPtrDbrY)),
            0xD2 => self.cmp(Mode(DirPtrDbr)),
            0xD3 => self.cmp(Mode(StackPtrDbrY)),
            0xD5 => self.cmp(Mode(DirX)),
            0xD7 => self.cmp(Mode(DirPtrY)),
            0xD9 => self.cmp(Mode(AbsY)),
            0xDD => self.cmp(Mode(AbsX)),
            0xDF => self.cmp(Mode(LongX)),

            0xE0 => self.cpx(Imm),
            0xE4 => self.cpx(Mode(Dir)),
            0xEC => self.cpx(Mode(Abs)),
            0xC0 => self.cpy(Imm),
            0xC4 => self.cpy(Mode(Dir)),
            0xCC => self.cpy(Mode(Abs)),

            0x3A => self.dec(Acc),
            0xC6 => self.dec(Mode(Dir)),
            0xCE => self.dec(Mode(Abs)),
            0xD6 => self.dec(Mode(DirX)),
            0xDE => self.dec(Mode(AbsX)),
            0xCA => self.dex(),
            0x88 => self.dey(),
            0x1A => self.inc(Acc),
            0xE6 => self.inc(Mode(Dir)),
            0xEE => self.inc(Mode(Abs)),
            0xF6 => self.inc(Mode(DirX)),
            0xFE => self.inc(Mode(AbsX)),
            0xE8 => self.inx(),
            0xC8 => self.iny(),

            0x21 => self.and(Mode(DirPtrXDbr)),
            0x23 => self.and(Mode(Stack)),
            0x25 => self.and(Mode(Dir)),
            0x27 => self.and(Mode(DirPtr)),
            0x29 => self.and(Imm),
            0x2D => self.and(Mode(Abs)),
            0x2F => self.and(Mode(Long)),
            0x31 => self.and(Mode(DirPtrDbrY)),
            0x32 => self.and(Mode(DirPtrDbr)),
            0x33 => self.and(Mode(StackPtrDbrY)),
            0x35 => self.and(Mode(DirX)),
            0x37 => self.and(Mode(DirPtrY)),
            0x39 => self.and(Mode(AbsY)),
            0x3D => self.and(Mode(AbsX)),
            0x3F => self.and(Mode(LongX)),

            0x41 => self.eor(Mode(DirPtrXDbr)),
            0x43 => self.eor(Mode(Stack)),
            0x45 => self.eor(Mode(Dir)),
            0x47 => self.eor(Mode(DirPtr)),
            0x49 => self.eor(Imm),
            0x4D => self.eor(Mode(Abs)),
            0x4F => self.eor(Mode(Long)),
            0x51 => self.eor(Mode(DirPtrDbrY)),
            0x52 => self.eor(Mode(DirPtrDbr)),
            0x53 => self.eor(Mode(StackPtrDbrY)),
            0x55 => self.eor(Mode(DirX)),
            0x57 => self.eor(Mode(DirPtrY)),
            0x59 => self.eor(Mode(AbsY)),
            0x5D => self.eor(Mode(AbsX)),
            0x5F => self.eor(Mode(LongX)),

            0x01 => self.ora(Mode(DirPtrXDbr)),
            0x03 => self.ora(Mode(Stack)),
            0x05 => self.ora(Mode(Dir)),
            0x07 => self.ora(Mode(DirPtr)),
            0x09 => self.ora(Imm),
            0x0D => self.ora(Mode(Abs)),
            0x0F => self.ora(Mode(Long)),
            0x11 => self.ora(Mode(DirPtrDbrY)),
            0x12 => self.ora(Mode(DirPtrDbr)),
            0x13 => self.ora(Mode(StackPtrDbrY)),
            0x15 => self.ora(Mode(DirX)),
            0x17 => self.ora(Mode(DirPtrY)),
            0x19 => self.ora(Mode(AbsY)),
            0x1D => self.ora(Mode(AbsX)),
            0x1F => self.ora(Mode(LongX)),

            0x24 => self.bit(Mode(Dir)),
            0x2C => self.bit(Mode(Abs)),
            0x34 => self.bit(Mode(DirX)),
            0x3C => self.bit(Mode(AbsX)),
            0x89 => self.bit(Imm),

            0x14 => self.trb(Mode(Dir)),
            0x1C => self.trb(Mode(Abs)),
            0x04 => self.tsb(Mode(Dir)),
            0x0C => self.tsb(Mode(Abs)),

            0x06 => self.asl(Mode(Dir)),
            0x0A => self.asl(Acc),
            0x0E => self.asl(Mode(Abs)),
            0x16 => self.asl(Mode(DirX)),
            0x1E => self.asl(Mode(AbsX)),

            0x46 => self.lsr(Mode(Dir)),
            0x4A => self.lsr(Acc),
            0x4E => self.lsr(Mode(Abs)),
            0x56 => self.lsr(Mode(DirX)),
            0x5E => self.lsr(Mode(AbsX)),

            0x26 => self.rol(Mode(Dir)),
            0x2A => self.rol(Acc),
            0x2E => self.rol(Mode(Abs)),
            0x36 => self.rol(Mode(DirX)),
            0x3E => self.rol(Mode(AbsX)),

            0x66 => self.ror(Mode(Dir)),
            0x6A => self.ror(Acc),
            0x6E => self.ror(Mode(Abs)),
            0x76 => self.ror(Mode(DirX)),
            0x7E => self.ror(Mode(AbsX)),

            0x90 => self.branch(PFlags::C, false),  // BCC
            0xB0 => self.branch(PFlags::C, true),   // BCS
            0xF0 => self.branch(PFlags::Z, true),   // BEQ
            0x30 => self.branch(PFlags::N, true),   // BMI
            0xD0 => self.branch(PFlags::Z, false),  // BNE
            0x10 => self.branch(PFlags::N, false),  // BPL
            0x80 => self.branch(PFlags::default(), true),  // BRA
            0x50 => self.branch(PFlags::V, false),  // BVC
            0x70 => self.branch(PFlags::V, true),   // BVS

            0x82 => self.brl(),

            0x4C => self.jmp(JumpAddrMode::Abs),
            0x5C => self.jmp(JumpAddrMode::Long),
            0x6C => self.jmp(JumpAddrMode::AbsPtrPbr),
            0x7C => self.jmp(JumpAddrMode::AbsPtrXPbr),
            0xDC => self.jmp(JumpAddrMode::AbsPtr),
            0x22 => self.js(JumpAddrMode::Long),        // JSL
            0x20 => self.js(JumpAddrMode::Abs),         // JSR
            0xFC => self.js(JumpAddrMode::AbsPtrXPbr),  // JSR

            0x6B => self.rtl(),
            0x60 => self.rts(),

            0x00 => self.brk(),
            0x02 => self.cop(),

            0x40 => self.rti(),

            0x18 => self.flag(PFlags::C, false),    // CLC
            0xD8 => self.flag(PFlags::D, false),    // CLD
            0x58 => self.flag(PFlags::I, false),    // CLI
            0xB8 => self.flag(PFlags::V, false),    // CLV
            0x38 => self.flag(PFlags::C, true),     // SEC
            0xF8 => self.flag(PFlags::D, true),     // SED
            0x78 => self.flag(PFlags::I, true),     // SEI

            0xC2 => self.rep(),
            0xE2 => self.sep(),

            0xA1 => self.lda(Mode(DirPtrXDbr)),
            0xA3 => self.lda(Mode(Stack)),
            0xA5 => self.lda(Mode(Dir)),
            0xA7 => self.lda(Mode(DirPtr)),
            0xA9 => self.lda(Imm),
            0xAD => self.lda(Mode(Abs)),
            0xAF => self.lda(Mode(Long)),
            0xB1 => self.lda(Mode(DirPtrDbrY)),
            0xB2 => self.lda(Mode(DirPtrDbr)),
            0xB3 => self.lda(Mode(StackPtrDbrY)),
            0xB5 => self.lda(Mode(DirX)),
            0xB7 => self.lda(Mode(DirPtrY)),
            0xB9 => self.lda(Mode(AbsY)),
            0xBD => self.lda(Mode(AbsX)),
            0xBF => self.lda(Mode(LongX)),

            0xA2 => self.ldx(Imm),
            0xA6 => self.ldx(Mode(Dir)),
            0xAE => self.ldx(Mode(Abs)),
            0xB6 => self.ldx(Mode(DirY)),
            0xBE => self.ldx(Mode(AbsY)),

            0xA0 => self.ldy(Imm),
            0xA4 => self.ldy(Mode(Dir)),
            0xAC => self.ldy(Mode(Abs)),
            0xB4 => self.ldy(Mode(DirX)),
            0xBC => self.ldy(Mode(AbsX)),

            0x81 => self.sta(Mode(DirPtrXDbr)),
            0x83 => self.sta(Mode(Stack)),
            0x85 => self.sta(Mode(Dir)),
            0x87 => self.sta(Mode(DirPtr)),
            0x8D => self.sta(Mode(Abs)),
            0x8F => self.sta(Mode(Long)),
            0x91 => self.sta(Mode(DirPtrDbrY)),
            0x92 => self.sta(Mode(DirPtrDbr)),
            0x93 => self.sta(Mode(StackPtrDbrY)),
            0x95 => self.sta(Mode(DirX)),
            0x97 => self.sta(Mode(DirPtrY)),
            0x99 => self.sta(Mode(AbsY)),
            0x9D => self.sta(Mode(AbsX)),
            0x9F => self.sta(Mode(LongX)),

            0x86 => self.stx(Mode(Dir)),
            0x8E => self.stx(Mode(Abs)),
            0x96 => self.stx(Mode(DirY)),

            0x84 => self.sty(Mode(Dir)),
            0x8C => self.sty(Mode(Abs)),
            0x94 => self.sty(Mode(DirX)),

            0x64 => self.stz(Mode(Dir)),
            0x74 => self.stz(Mode(DirX)),
            0x9C => self.stz(Mode(Abs)),
            0x9E => self.stz(Mode(AbsX)),

            0x54 => self.mvn(),
            0x44 => self.mvp(),

            0xEA => self.nop(),
            0x42 => self.wdm(),

            0xF4 => self.pe(Imm),       // PEA
            0xD4 => self.pe(Mode(Dir)), // PEI
            0x62 => self.per(),

            0x48 => self.ph(self.a, self.is_m_set()),     // PHA
            0xDA => self.ph(self.x, self.is_x_set()),     // PHX
            0x5A => self.ph(self.y, self.is_x_set()),     // PHY
            0x68 => self.a = self.pl(self.is_m_set()),    // PLA
            0xFA => self.x = self.pl(self.is_x_set()),    // PLX
            0x7A => self.y = self.pl(self.is_x_set()),    // PLY

            0x8B => self.ph(self.db as u16, true),       // PHB
            0x0B => self.ph(self.dp, false),             // PHD
            0x4B => self.ph(self.pb as u16, true),       // PHK
            0x08 => self.ph(self.p.bits() as u16, true), // PHP
            0xAB => self.db = self.pl(false) as u8,      // PLB
            0x2B => self.dp = self.pl(true),             // PLD
            0x28 => self.p = PFlags::from_bits_truncate(self.pl(false) as u8),    // PLP

            0xDB => self.stp(),
            0xCB => self.wai(),

            0xAA => self.x = self.transfer(self.a, self.x, self.is_x_set(), true),  // TAX
            0xA8 => self.y = self.transfer(self.a, self.y, self.is_x_set(), true),  // TAY
            0xBA => self.x = self.transfer(self.s, self.x, self.is_x_set(), true),  // TSX
            0x8A => self.a = self.transfer(self.x, self.a, self.is_m_set(), true),  // TXA
            0x9A => self.s = self.transfer(self.x, 0, self.is_x_set() && !self.is_e_set(), false),  // TXS
            0x9B => self.y = self.transfer(self.x, self.y, self.is_x_set(), true),  // TXY
            0x98 => self.a = self.transfer(self.y, self.a, self.is_m_set(), true),  // TYA
            0xBB => self.x = self.transfer(self.y, self.x, self.is_x_set(), true),  // TYX

            0x5B => self.dp = self.transfer(self.a, 0, false, true),                // TCD
            0x1B => self.s = self.transfer(self.a, 0x0100, self.is_e_set(), false), // TCS
            0x7B => self.a = self.transfer(self.dp, 0, false, true),                // TDC
            0x3B => self.a = self.transfer(self.s, 0, false, true),                 // TSC

            0xEB => self.xba(),
            0xFB => self.xce(),
        }
    }

    // Clock
    fn clock_inc(&mut self, cycles: usize) {
        for _ in 0..cycles {

        }
    }
}

// Internal: Data instructions
impl CPU {
    // TODO: bcd mode.
    fn adc(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, self.is_m_set());
        let result = self.a.wrapping_add(op).wrapping_add((self.p & PFlags::C).bits() as u16);

        self.a = if self.is_m_set() {
            let result8 = result & 0xFF;
            let full_wraparound = (result8 == self.a) && (op != 0);
            self.p.set(PFlags::N, test_bit!(result8, 7));
            self.p.set(PFlags::V, ((result8 as i8) < (self.a as i8)) || full_wraparound);
            self.p.set(PFlags::Z, result8 == 0);
            self.p.set(PFlags::C, (result8 < self.a) || full_wraparound);

            result8
        } else {
            let full_wraparound = (result == self.a) && (op != 0);
            self.p.set(PFlags::N, test_bit!(result, 15));
            self.p.set(PFlags::V, ((result as i16) < (self.a as i16)) || full_wraparound);
            self.p.set(PFlags::Z, result == 0);
            self.p.set(PFlags::C, (result < self.a) || full_wraparound);

            result
        };
    }

    fn sbc(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, self.is_m_set());
        let result = self.a.wrapping_sub(op).wrapping_sub(1).wrapping_add((self.p & PFlags::C).bits() as u16);

        self.a = if self.is_m_set() {
            let result8 = result & 0xFF;
            let full_wraparound = (result8 == self.a) && (op != 0);
            self.p.set(PFlags::N, test_bit!(result8, 7));
            self.p.set(PFlags::V, ((result8 as i8) > (self.a as i8)) || full_wraparound);
            self.p.set(PFlags::Z, result8 == 0);
            self.p.set(PFlags::C, (result8 > self.a) || full_wraparound);

            result8
        } else {
            let full_wraparound = (result == self.a) && (op != 0);
            self.p.set(PFlags::N, test_bit!(result, 15));
            self.p.set(PFlags::V, ((result as i16) > (self.a as i16)) || full_wraparound);
            self.p.set(PFlags::Z, result == 0);
            self.p.set(PFlags::C, (result > self.a) || full_wraparound);

            result
        };
    }

    fn cmp(&mut self, data_mode: DataMode) {
        self.compare(data_mode, self.a, PFlags::M);
    }

    fn cpx(&mut self, data_mode: DataMode) {
        self.compare(data_mode, self.x, PFlags::X);
    }

    fn cpy(&mut self, data_mode: DataMode) {
        self.compare(data_mode, self.y, PFlags::X);
    }

    fn dec(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let result = op.wrapping_sub(1);

        let data = self.set_nz(result, self.is_m_set());

        self.write_op(data, write_mode, self.is_m_set());
    }

    fn dex(&mut self) {
        let result = self.x.wrapping_sub(1);

        self.x = self.set_nz(result, self.is_x_set());
    }

    fn dey(&mut self) {
        let result = self.y.wrapping_sub(1);

        self.y = self.set_nz(result, self.is_x_set());
    }

    fn inc(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let result = op.wrapping_add(1);

        let data = self.set_nz(result, self.is_m_set());

        self.write_op(data, write_mode, self.is_m_set());
    }

    fn inx(&mut self) {
        let result = self.x.wrapping_add(1);

        self.x = self.set_nz(result, self.is_x_set());
    }

    fn iny(&mut self) {
        let result = self.y.wrapping_add(1);

        self.y = self.set_nz(result, self.is_x_set());
    }

    fn and(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, self.is_m_set());
        let result = self.a & op;

        self.a = self.set_nz(result, self.is_m_set());
    }

    fn eor(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, self.is_m_set());
        let result = self.a ^ op;

        self.a = self.set_nz(result, self.is_m_set());
    }

    fn ora(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, self.is_m_set());
        let result = self.a | op;

        self.a = self.set_nz(result, self.is_m_set());
    }

    fn bit(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, self.is_m_set());
        let result = self.a & op;

        if self.is_m_set() {
            let result8 = result & 0xFF;
            self.p.set(PFlags::Z, result8 == 0);

            match data_mode {
                DataMode::Imm => {},
                _ => {
                    self.p.set(PFlags::N, test_bit!(op, 7));
                    self.p.set(PFlags::V, test_bit!(op, 6));
                }
            }
        } else {
            self.p.set(PFlags::Z, result == 0);

            match data_mode {
                DataMode::Imm => {},
                _ => {
                    self.p.set(PFlags::N, test_bit!(op, 15));
                    self.p.set(PFlags::V, test_bit!(op, 14));
                }
            }
        }
    }

    fn trb(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let result = self.a & op;

        self.set_z(result);

        self.clock_inc(INTERNAL_OP);

        let write_data = op & (!self.a);
        self.write_op(write_data, write_mode, self.is_m_set());
    }

    fn tsb(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let result = self.a & op;

        self.set_z(result);

        self.clock_inc(INTERNAL_OP);

        let write_data = op | self.a;
        self.write_op(write_data, write_mode, self.is_m_set());
    }

    fn asl(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let result = op << 1;

        self.p.set(PFlags::C, if self.is_m_set() {
            test_bit!(op, 7)
        } else {
            test_bit!(op, 15)
        });

        self.clock_inc(INTERNAL_OP);

        let write_data = self.set_nz(result, self.is_m_set());
        self.write_op(write_data, write_mode, self.is_m_set());
    }

    fn lsr(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let result = op >> 1;

        self.p.set(PFlags::C, test_bit!(op, 0));

        self.clock_inc(INTERNAL_OP);

        let write_data = self.set_nz(result, self.is_m_set());
        self.write_op(write_data, write_mode, self.is_m_set());
    }

    fn rol(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let result = (op << 1) | ((self.p & PFlags::C).bits() as u16);

        self.p.set(PFlags::C, if self.is_m_set() {
            test_bit!(op, 7)
        } else {
            test_bit!(op, 15)
        });

        self.clock_inc(INTERNAL_OP);

        let write_data = self.set_nz(result, self.is_m_set());
        self.write_op(write_data, write_mode, self.is_m_set());
    }

    fn ror(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, self.is_m_set());
        let carry = ((self.p & PFlags::C).bits() as u16) << (if self.is_m_set() {7} else {15});
        let result = (op >> 1) | carry;

        self.p.set(PFlags::C, test_bit!(op, 0));

        self.clock_inc(INTERNAL_OP);

        let write_data = self.set_nz(result, self.is_m_set());
        self.write_op(write_data, write_mode, self.is_m_set());
    }
}

// Internal: Branch/Jump instructions
impl CPU {
    fn branch(&mut self, flag_check: PFlags, set: bool) {
        let imm = (self.fetch() as i8) as i16;

        if self.p.contains(flag_check) == set {
            let pc = self.pc.wrapping_add(imm as u16);

            if self.is_e_set() && (hi!(pc) != hi!(self.pc)) {
                self.clock_inc(INTERNAL_OP);
            }

            self.clock_inc(INTERNAL_OP);

            self.pc = pc;
        }
    }

    fn brl(&mut self) {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        self.clock_inc(INTERNAL_OP);

        self.pc = self.pc.wrapping_add(make16!(imm_hi, imm_lo));
    }

    fn jmp(&mut self, addr_mode: JumpAddrMode) {
        let addr = self.get_jump_addr(addr_mode);

        match addr {
            Addr::Full(a) => {
                self.pb = hi24!(a);
                self.pc = lo24!(a);
            },
            Addr::ZeroBank(a) => self.pc = a
        }
    }

    fn js(&mut self, addr_mode: JumpAddrMode) {
        let addr = self.get_jump_addr(addr_mode);

        let pc = self.pc.wrapping_sub(1);

        if addr_mode != JumpAddrMode::AbsPtrXPbr {
            self.clock_inc(INTERNAL_OP);
        }

        match addr {
            Addr::Full(a) => {
                self.stack_push(self.pb);

                self.pb = hi24!(a);
                self.pc = lo24!(a);
            },
            Addr::ZeroBank(a) => {
                self.pc = a
            }
        }

        self.stack_push(hi!(pc));
        self.stack_push(lo!(pc));
    }

    fn rtl(&mut self) {
        let pc_lo = self.stack_pop();
        let pc_hi = self.stack_pop();
        let pb = self.stack_pop();

        self.clock_inc(INTERNAL_OP * 2);

        self.pc = make16!(pc_hi, pc_lo).wrapping_add(1);
        self.pb = pb;
    }

    fn rts(&mut self) {
        let pc_lo = self.stack_pop();
        let pc_hi = self.stack_pop();

        self.clock_inc(INTERNAL_OP * 3);

        self.pc = make16!(pc_hi, pc_lo).wrapping_add(1);
    }

    fn brk(&mut self) {
        // TODO
    }

    fn cop(&mut self) {
        // TODO
    }

    fn rti(&mut self) {
        self.p = PFlags::from_bits_truncate(self.stack_pop());
        let pc_lo = self.stack_pop();
        let pc_hi = self.stack_pop();

        self.pc = make16!(pc_hi, pc_lo);

        self.clock_inc(INTERNAL_OP * 2);

        if !self.is_e_set() {
            self.pb = self.stack_pop();
        }
    }
}

// Internal: Misc ops
impl CPU {
    fn flag(&mut self, flag: PFlags, set: bool) {
        self.p.set(flag, set);
        self.clock_inc(INTERNAL_OP);
    }

    fn rep(&mut self) {
        let imm = self.fetch();

        self.p &= PFlags::from_bits_truncate(!imm);
        self.clock_inc(INTERNAL_OP);

        if self.is_e_set() {
            self.p |= PFlags::M | PFlags::X;
        }
    }

    fn sep(&mut self) {
        let imm = self.fetch();

        self.p |= PFlags::from_bits_truncate(imm);
        self.clock_inc(INTERNAL_OP);
    }

    fn nop(&mut self) {
        self.clock_inc(INTERNAL_OP);
    }

    fn wdm(&mut self) {
        self.pc = self.pc.wrapping_add(1);
        self.clock_inc(INTERNAL_OP);
    }

    fn stp(&mut self) {
        // TODO
        self.clock_inc(INTERNAL_OP * 2);
    }

    fn wai(&mut self) {
        // TODO
        self.clock_inc(INTERNAL_OP * 2);
    }

    fn xba(&mut self) {
        let b = hi!(self.a);
        let a = lo!(self.a);

        let _ = self.set_nz(b as u16, true);

        self.clock_inc(INTERNAL_OP * 2);

        self.a = make16!(a, b);
    }

    fn xce(&mut self) {
        let c_set = self.p.contains(PFlags::C);
        let e_set = self.is_e_set();
        self.pe.set(PFlags::E, c_set);
        self.p.set(PFlags::C, e_set);

        self.clock_inc(INTERNAL_OP);

        if c_set {
            self.p.insert(PFlags::M | PFlags::X);
            self.x = set_hi!(self.x, 0);
            self.y = set_hi!(self.y, 0);
            self.s = set_hi!(self.s, 1);
        }
    }
}

// Internal: Data moving ops
impl CPU {
    fn lda(&mut self, data_mode: DataMode) {
        let data = self.read_op(data_mode, self.is_m_set());

        self.a = self.set_nz(data, self.is_m_set());
    }

    fn ldx(&mut self, data_mode: DataMode) {
        let data = self.read_op(data_mode, self.is_x_set());

        self.x = self.set_nz(data, self.is_x_set());
    }

    fn ldy(&mut self, data_mode: DataMode) {
        let data = self.read_op(data_mode, self.is_x_set());

        self.y = self.set_nz(data, self.is_x_set());
    }

    fn sta(&mut self, data_mode: DataMode) {
        self.write_op(self.a, data_mode, self.is_m_set());
    }

    fn stx(&mut self, data_mode: DataMode) {
        self.write_op(self.x, data_mode, self.is_x_set());
    }

    fn sty(&mut self, data_mode: DataMode) {
        self.write_op(self.y, data_mode, self.is_x_set());
    }

    fn stz(&mut self, data_mode: DataMode) {
        self.write_op(0, data_mode, self.is_m_set());
    }

    fn mvn(&mut self) {
        let src_bank = self.fetch();
        let dst_bank = self.fetch();

        let src_addr = make24!(src_bank, self.x);
        let dst_addr = make24!(dst_bank, self.y);

        let byte = self.read_data(src_addr);
        self.write_data(dst_addr, byte);

        self.x = self.x.wrapping_add(1);
        self.y = self.y.wrapping_add(1);

        self.a = self.a.wrapping_sub(1);

        self.clock_inc(INTERNAL_OP * 2);

        if self.a != 0xFFFF {
            self.pc = self.pc.wrapping_sub(3);
        }
    }

    fn mvp(&mut self) {
        let src_bank = self.fetch();
        let dst_bank = self.fetch();

        let src_addr = make24!(src_bank, self.x);
        let dst_addr = make24!(dst_bank, self.y);

        let byte = self.read_data(src_addr);
        self.write_data(dst_addr, byte);

        self.x = self.x.wrapping_sub(1);
        self.y = self.y.wrapping_sub(1);

        self.a = self.a.wrapping_sub(1);

        self.clock_inc(INTERNAL_OP * 2);

        if self.a != 0xFFFF {
            self.pc = self.pc.wrapping_sub(3);
        }
    }

    fn pe(&mut self, data_mode: DataMode) {
        let data = self.read_op(data_mode, true);

        self.stack_push(hi!(data));
        self.stack_push(lo!(data));
    }

    fn per(&mut self) {
        let imm = self.immediate(true);

        let data = self.pc.wrapping_add(imm);

        self.clock_inc(INTERNAL_OP);

        self.stack_push(hi!(data));
        self.stack_push(lo!(data));
    }

    fn ph(&mut self, reg: u16, byte: bool) {
        if byte {
            self.stack_push(reg as u8);
        } else {
            self.stack_push(hi!(reg));
            self.stack_push(lo!(reg));
        }

        self.clock_inc(INTERNAL_OP);
    }

    fn pl(&mut self, byte: bool) -> u16 {
        let reg = if byte {
            self.stack_pop() as u16
        } else {
            let lo = self.stack_pop();
            let hi = self.stack_pop();
            make16!(hi, lo)
        };

        self.clock_inc(INTERNAL_OP);

        self.set_nz(reg, byte)
    }

    fn transfer(&mut self, from: u16, to: u16, byte: bool, set_flags: bool) -> u16 {
        let result = if set_flags {
            self.set_nz(from, byte)
        } else {
            from
        };

        self.clock_inc(INTERNAL_OP);

        if byte {
            set_lo!(to, result)
        } else {
            result
        }
    }
}

// Internal: Data and Flag setting Micro-ops
impl CPU {
    // Set N if high bit is 1, set Z if result is zero. Return 8 or 16 bit result.
    fn set_nz(&mut self, result: u16, byte: bool) -> u16 {
        if byte {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, test_bit!(result8, 7));
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, test_bit!(result, 15));
            self.p.set(PFlags::Z, result == 0);

            result
        }
    }

    // Set Z if result is zero. Always checks M flag.
    fn set_z(&mut self, result: u16) {
        if self.is_m_set() {
            let result8 = result & 0xFF;
            self.p.set(PFlags::Z, result8 == 0);
        } else {
            self.p.set(PFlags::Z, result == 0);
        }
    }

    // Compare register with operand, and set flags accordingly.
    fn compare(&mut self, data_mode: DataMode, reg: u16, flag_check: PFlags) {
        let op = self.read_op(data_mode, self.p.contains(flag_check));
        let result = reg.wrapping_sub(op);

        if self.p.contains(flag_check) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, test_bit!(result8, 7));
            self.p.set(PFlags::Z, result8 == 0);
            self.p.set(PFlags::C, result8 >= reg);
        } else {
            self.p.set(PFlags::N, test_bit!(result, 15));
            self.p.set(PFlags::Z, result == 0);
            self.p.set(PFlags::C, result >= reg);
        }
    }

    fn is_m_set(&self) -> bool {
        self.p.contains(PFlags::M)
    }

    fn is_x_set(&self) -> bool {
        self.p.contains(PFlags::X)
    }

    fn is_e_set(&self) -> bool {
        self.pe.contains(PFlags::E)
    }
}

// Internal: Memory and Addressing Micro-ops
impl CPU {
    // Read a byte from the (data) bus.
    fn read_data(&mut self, addr: u32) -> u8 {
        let (data, cycles) = self.mem.read(addr);
        self.clock_inc(cycles);
        data
    }

    // Write a byte to the (data) bus.
    fn write_data(&mut self, addr: u32, data: u8) {
        let cycles = self.mem.write(addr, data);
        self.clock_inc(cycles);
    }

    // Pop a byte from the stack.
    fn stack_pop(&mut self) -> u8 {
        self.s = self.s.wrapping_add(1);
        self.read_data(self.s as u32)
    }

    // Push a byte to the stack.
    fn stack_push(&mut self, data: u8) {
        self.write_data(self.s as u32, data);
        self.s = self.s.wrapping_sub(1);
    }

    // Read one or two bytes.
    fn read_addr(&mut self, addr: Addr, byte: bool) -> u16 {
        use self::Addr::*;

        match addr {
            Full(a) => {
                let data_lo = self.read_data(a);

                let data_hi = if !byte {
                    self.read_data(a.wrapping_add(1))
                } else { 0 };
                
                make16!(data_hi, data_lo)
            },
            ZeroBank(a) => {
                let addr_lo = make24!(0, a);
                let data_lo = self.read_data(addr_lo);

                let data_hi = if !byte {
                    let addr_hi = make24!(0, a.wrapping_add(1));
                    self.read_data(addr_hi)
                } else { 0 };
                
                make16!(data_hi, data_lo)
            }
        }
    }

    // Write one or two bytes (based on the value of the M or X flag).
    fn write_addr(&mut self, data: u16, addr: Addr, byte: bool) {
        use self::Addr::*;

        match addr {
            Full(a) => {
                self.write_data(a, lo!(data));

                if !byte {
                    self.write_data(a.wrapping_add(1), hi!(data));
                }
            },
            ZeroBank(a) => {
                let addr_lo = make24!(0, a);
                self.write_data(addr_lo, lo!(data));

                if !byte {
                    let addr_hi = make24!(0, a.wrapping_add(1));
                    self.write_data(addr_hi, hi!(data));
                }
            }
        }
    }

    // Fetch a byte from the PC.
    fn fetch(&mut self) -> u8 {
        let data = self.read_data(make24!(self.pb, self.pc));
        self.pc = self.pc.wrapping_add(1);
        data
    }

    // Get an operand using the specified data mode.
    fn read_op(&mut self, data_mode: DataMode, byte: bool) -> u16 {
        use self::DataMode::*;

        match data_mode {
            Imm => self.immediate(byte),
            Acc => self.a,
            Mode(m) => {
                let addr = self.get_data_addr(m);
                self.read_addr(addr, byte)
            },
            Known(_) => unreachable!() // In practice we never read from known addresses.
        }
    }

    // Get an operand using the specified data mode and return the address if an addressing mode was used.
    fn read_op_and_addr_mode(&mut self, data_mode: DataMode, byte: bool) -> (u16, DataMode) {
        use self::DataMode::*;

        match data_mode {
            Imm => unreachable!(),  // We can't write back to immediate data.
            Acc => (self.a, Acc),
            Mode(m) => {
                let addr = self.get_data_addr(m);
                (self.read_addr(addr, byte), Known(addr))
            },
            Known(_) => unreachable!() // In practice we never read from known addresses.
        }
    }

    // Set an operand using the specified addressing mode.
    fn write_op(&mut self, data: u16, data_mode: DataMode, byte: bool) {
        use self::DataMode::*;

        match data_mode {
            Imm => unreachable!(),  // We can't write to immediate data.
            Acc => self.a = data,
            Mode(m) => {
                let addr = self.get_data_addr(m);
                self.write_addr(data, addr, byte);
            },
            Known(a) => self.write_addr(data, a, byte)
        }
    }

    // Get an address of data using the specified addressing mode.
    fn get_data_addr(&mut self, addr_mode: DataAddrMode) -> Addr {
        use self::DataAddrMode::*;

        match addr_mode {
            Abs             => self.absolute(),
            AbsX            => self.absolute_x(),
            AbsY            => self.absolute_y(),

            Dir             => self.direct(),
            DirX            => self.direct_x(),
            DirY            => self.direct_y(),
            DirPtrDbr       => self.direct_ptr_dbr(),
            DirPtrXDbr      => self.direct_ptr_x_dbr(),
            DirPtrDbrY      => self.direct_ptr_dbr_y(),
            DirPtr          => self.direct_ptr(),
            DirPtrY         => self.direct_ptr_y(),
            
            Long            => self.long(),
            LongX           => self.long_x(),
            Stack           => self.stack(),
            StackPtrDbrY    => self.stack_ptr_dbr_y()
        }
    }

    // Get an address of a branch using the specified addressing mode.
    fn get_jump_addr(&mut self, addr_mode: JumpAddrMode) -> Addr {
        use self::JumpAddrMode::*;

        match addr_mode {
            Abs         => self.absolute_pbr(),
            AbsPtrPbr   => self.absolute_ptr_pbr(),
            AbsPtrXPbr  => self.absolute_ptr_x_pbr(),
            AbsPtr      => self.absolute_ptr(),
            Long        => self.long()
        }
    }

    // Addressing modes:

    // #$vvvv
    fn immediate(&mut self, byte: bool) -> u16 {
        let imm_lo = self.fetch();

        let imm_hi = if !byte {
            self.fetch()
        } else { 0 };

        make16!(imm_hi, imm_lo)
    }

    // $vvvv
    fn absolute(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        Addr::Full(make24!(self.db, imm_hi, imm_lo))
    }

    // $vvvv, X
    fn absolute_x(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let abs_addr = make24!(self.db, imm_hi, imm_lo);
        let addr = abs_addr.wrapping_add(self.x as u32);

        if !self.is_x_set() || (self.is_x_set() && (abs_addr < addr)) {
            self.clock_inc(INTERNAL_OP);
        }

        Addr::Full(addr)
    }

    // $vvvv, Y
    fn absolute_y(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let abs_addr = make24!(self.db, imm_hi, imm_lo);
        let addr = abs_addr.wrapping_add(self.y as u32);

        if !self.is_x_set() || (self.is_x_set() && (abs_addr < addr)) {
            self.clock_inc(INTERNAL_OP);
        }

        Addr::Full(addr)
    }

    // $vv
    fn direct(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        /*let addr = if self.is_e_set() && (lo!(self.dp) == 0) {
            set_lo!(self.dp, imm)
        } else {
            self.dp.wrapping_add(imm as u16)
        };*/
        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        Addr::ZeroBank(self.dp.wrapping_add(imm))
    }

    // $vv, X
    fn direct_x(&mut self) -> Addr {
        let imm = self.fetch();

        let addr = if self.is_e_set() && (lo!(self.dp) == 0) {
            let addr_lo = (self.x as u8).wrapping_add(imm);
            set_lo!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.x).wrapping_add(imm as u16)
        };

        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        self.clock_inc(INTERNAL_OP);

        Addr::ZeroBank(addr)
    }

    // $vv, Y
    fn direct_y(&mut self) -> Addr {
        let imm = self.fetch();

        let addr = if self.is_e_set() && (lo!(self.dp) == 0) {
            let addr_lo = (self.y as u8).wrapping_add(imm);
            set_lo!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.y).wrapping_add(imm as u16)
        };

        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        self.clock_inc(INTERNAL_OP);

        Addr::ZeroBank(addr)
    }

    // ($vv)
    fn direct_ptr_dbr(&mut self) -> Addr {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.is_e_set() && (lo!(self.dp) == 0) {
            (set_lo!(self.dp, imm), set_lo!(self.dp, imm.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        Addr::Full(make24!(self.db, addr_hi, addr_lo))
    }

    // ($vv, X)
    fn direct_ptr_x_dbr(&mut self) -> Addr {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.is_e_set() && (lo!(self.dp) == 0) {
            let ptr_addr_lo = (self.x as u8).wrapping_add(imm);
            (set_lo!(self.dp, ptr_addr_lo), set_lo!(self.dp, ptr_addr_lo.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(self.x).wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        self.clock_inc(INTERNAL_OP);

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        Addr::Full(make24!(self.db, addr_hi, addr_lo))
    }

    // ($vv), Y
    fn direct_ptr_dbr_y(&mut self) -> Addr {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.is_e_set() && (lo!(self.dp) == 0) {
            (set_lo!(self.dp, imm), set_lo!(self.dp, imm.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        let addr = make24!(self.db, addr_hi, addr_lo);
        let final_addr = addr.wrapping_add(self.y as u32);

        if !self.is_x_set() || (self.is_x_set() && (addr < final_addr)) {
            self.clock_inc(INTERNAL_OP);
        }

        Addr::Full(final_addr)
    }

    // [$vv]
    fn direct_ptr(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        let ptr = self.dp.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_mid = make24!(0, ptr.wrapping_add(1));
        let ptr_hi = make24!(0, ptr.wrapping_add(2));

        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        let addr_lo = self.read_data(ptr_lo);
        let addr_mid = self.read_data(ptr_mid);
        let addr_hi = self.read_data(ptr_hi);

        Addr::Full(make24!(addr_hi, addr_mid, addr_lo))
    }

    // [$vv], Y
    fn direct_ptr_y(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        let ptr = self.dp.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_mid = make24!(0, ptr.wrapping_add(1));
        let ptr_hi = make24!(0, ptr.wrapping_add(2));

        if lo!(self.dp) != 0 {
            self.clock_inc(INTERNAL_OP);
        }

        let addr_lo = self.read_data(ptr_lo);
        let addr_mid = self.read_data(ptr_mid);
        let addr_hi = self.read_data(ptr_hi);

        Addr::Full(make24!(addr_hi, addr_mid, addr_lo).wrapping_add(self.y as u32))
    }

    // $vv, s
    fn stack(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        let addr = self.s.wrapping_add(imm);

        self.clock_inc(INTERNAL_OP);

        Addr::ZeroBank(addr)
    }

    // ($vv, s), Y
    fn stack_ptr_dbr_y(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        let ptr = self.s.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_hi = make24!(0, ptr.wrapping_add(1));

        self.clock_inc(INTERNAL_OP * 2);

        let addr_lo = self.read_data(ptr_lo);
        let addr_hi = self.read_data(ptr_hi);

        Addr::Full(make24!(self.db, addr_hi, addr_lo).wrapping_add(self.y as u32))
    }

    // $vvvvvv
    fn long(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_mid = self.fetch();
        let imm_hi = self.fetch();

        Addr::Full(make24!(imm_hi, imm_mid, imm_lo))
    }

    // $vvvvvv, X
    fn long_x(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_mid = self.fetch();
        let imm_hi = self.fetch();

        Addr::Full(make24!(imm_hi, imm_mid, imm_lo).wrapping_add(self.x as u32))
    }

    // $vvvv
    fn absolute_pbr(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        Addr::ZeroBank(make16!(imm_hi, imm_lo))
    }

    // ($vvvv)
    fn absolute_ptr_pbr(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let ptr = make16!(imm_lo, imm_hi);

        let ptr_lo = make24!(0, ptr);
        let ptr_hi = make24!(0, ptr.wrapping_add(1));

        let addr_lo = self.read_data(ptr_lo);
        let addr_hi = self.read_data(ptr_hi);

        Addr::ZeroBank(make16!(addr_hi, addr_lo))
    }

    // ($vvvv, X)
    fn absolute_ptr_x_pbr(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let ptr = make16!(imm_lo, imm_hi).wrapping_add(self.x);

        let ptr_lo = make24!(self.pb, ptr);
        let ptr_hi = make24!(self.pb, ptr.wrapping_add(1));

        self.clock_inc(INTERNAL_OP);

        let addr_lo = self.read_data(ptr_lo);
        let addr_hi = self.read_data(ptr_hi);

        Addr::ZeroBank(make16!(addr_hi, addr_lo))
    }

    // [$vvvv]
    fn absolute_ptr(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let ptr = make16!(imm_lo, imm_hi);

        let ptr_lo = make24!(0, ptr);
        let ptr_mid = make24!(0, ptr.wrapping_add(1));
        let ptr_hi = make24!(0, ptr.wrapping_add(2));

        let addr_lo = self.read_data(ptr_lo);
        let addr_mid = self.read_data(ptr_mid);
        let addr_hi = self.read_data(ptr_hi);

        Addr::Full(make24!(addr_hi, addr_mid, addr_lo))
    }
}