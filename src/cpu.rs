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

// Data modes
#[derive(Clone, Copy)]
enum DataMode {
    Imm,                // Immediate data after the instruction
    Acc,                // Accumulator data
    Mode(AddrMode),     // Find the address using the given Addressing mode
    Known(Addr)         // Use the address provided
}

// Addressing modes
#[derive(Clone, Copy)]
enum AddrMode {
    Abs,
    AbsX,
    AbsY,

    Dir,
    DirX,
    DirY,
    DirPtrDBR,
    DirPtrXDBR,
    DirPtrDBRY,
    DirPtr,
    DirPtrY,

    Long,
    LongX,
    Stack,
    StackPtrDBRY
}

// Addresses
#[derive(Clone, Copy)]
enum Addr {
    Full(u32),      // A full, wrapping, 24-bit address.
    ZeroBank(u16)   // A 16-bit address that wraps at bank boundaries.
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
        use self::AddrMode::*;
        use self::DataMode::*;

        let instr = self.fetch();

        match instr {
            0x61 => self.adc(Mode(DirPtrXDBR)),
            0x63 => self.adc(Mode(Stack)),
            0x65 => self.adc(Mode(Dir)),
            0x67 => self.adc(Mode(DirPtr)),
            0x69 => self.adc(Imm),
            0x6D => self.adc(Mode(Abs)),
            0x6F => self.adc(Mode(Long)),
            0x71 => self.adc(Mode(DirPtrDBRY)),
            0x72 => self.adc(Mode(DirPtrDBR)),
            0x73 => self.adc(Mode(StackPtrDBRY)),
            0x75 => self.adc(Mode(DirX)),
            0x77 => self.adc(Mode(DirPtrY)),
            0x79 => self.adc(Mode(AbsY)),
            0x7D => self.adc(Mode(AbsX)),
            0x7F => self.adc(Mode(LongX)),

            0xE1 => self.sbc(Mode(DirPtrXDBR)),
            0xE3 => self.sbc(Mode(Stack)),
            0xE5 => self.sbc(Mode(Dir)),
            0xE7 => self.sbc(Mode(DirPtr)),
            0xE9 => self.sbc(Imm),
            0xED => self.sbc(Mode(Abs)),
            0xEF => self.sbc(Mode(Long)),
            0xF1 => self.sbc(Mode(DirPtrDBRY)),
            0xF2 => self.sbc(Mode(DirPtrDBR)),
            0xF3 => self.sbc(Mode(StackPtrDBRY)),
            0xF5 => self.sbc(Mode(DirX)),
            0xF7 => self.sbc(Mode(DirPtrY)),
            0xF9 => self.sbc(Mode(AbsY)),
            0xFD => self.sbc(Mode(AbsX)),
            0xFF => self.sbc(Mode(LongX)),

            0xC1 => self.cmp(Mode(DirPtrXDBR)),
            0xC3 => self.cmp(Mode(Stack)),
            0xC5 => self.cmp(Mode(Dir)),
            0xC7 => self.cmp(Mode(DirPtr)),
            0xC9 => self.cmp(Imm),
            0xCD => self.cmp(Mode(Abs)),
            0xCF => self.cmp(Mode(Long)),
            0xD1 => self.cmp(Mode(DirPtrDBRY)),
            0xD2 => self.cmp(Mode(DirPtrDBR)),
            0xD3 => self.cmp(Mode(StackPtrDBRY)),
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

            0x21 => self.and(Mode(DirPtrXDBR)),
            0x23 => self.and(Mode(Stack)),
            0x25 => self.and(Mode(Dir)),
            0x27 => self.and(Mode(DirPtr)),
            0x29 => self.and(Imm),
            0x2D => self.and(Mode(Abs)),
            0x2F => self.and(Mode(Long)),
            0x31 => self.and(Mode(DirPtrDBRY)),
            0x32 => self.and(Mode(DirPtrDBR)),
            0x33 => self.and(Mode(StackPtrDBRY)),
            0x35 => self.and(Mode(DirX)),
            0x37 => self.and(Mode(DirPtrY)),
            0x39 => self.and(Mode(AbsY)),
            0x3D => self.and(Mode(AbsX)),
            0x3F => self.and(Mode(LongX)),

            0x41 => self.eor(Mode(DirPtrXDBR)),
            0x43 => self.eor(Mode(Stack)),
            0x45 => self.eor(Mode(Dir)),
            0x47 => self.eor(Mode(DirPtr)),
            0x49 => self.eor(Imm),
            0x4D => self.eor(Mode(Abs)),
            0x4F => self.eor(Mode(Long)),
            0x51 => self.eor(Mode(DirPtrDBRY)),
            0x52 => self.eor(Mode(DirPtrDBR)),
            0x53 => self.eor(Mode(StackPtrDBRY)),
            0x55 => self.eor(Mode(DirX)),
            0x57 => self.eor(Mode(DirPtrY)),
            0x59 => self.eor(Mode(AbsY)),
            0x5D => self.eor(Mode(AbsX)),
            0x5F => self.eor(Mode(LongX)),

            0x01 => self.ora(Mode(DirPtrXDBR)),
            0x03 => self.ora(Mode(Stack)),
            0x05 => self.ora(Mode(Dir)),
            0x07 => self.ora(Mode(DirPtr)),
            0x09 => self.ora(Imm),
            0x0D => self.ora(Mode(Abs)),
            0x0F => self.ora(Mode(Long)),
            0x11 => self.ora(Mode(DirPtrDBRY)),
            0x12 => self.ora(Mode(DirPtrDBR)),
            0x13 => self.ora(Mode(StackPtrDBRY)),
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

            _ => unreachable!()
        }
    }
}

// Internal: Instructions
impl CPU {
    // TODO: bcd mode.
    fn adc(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, PFlags::M);
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

    fn sbc(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, PFlags::M);
        let result = self.a.wrapping_sub(op).wrapping_sub(1).wrapping_add((self.p & PFlags::C).bits() as u16);

        self.a = if self.p.contains(PFlags::M) {
            let result8 = result & 0xFF;
            let full_wraparound = (result8 == self.a) && (op != 0);
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::V, ((result8 as i8) > (self.a as i8)) || full_wraparound);
            self.p.set(PFlags::Z, result8 == 0);
            self.p.set(PFlags::C, (result8 > self.a) || full_wraparound);

            result8
        } else {
            let full_wraparound = (result == self.a) && (op != 0);
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::V, ((result as i16) > (self.a as i16)) || full_wraparound);
            self.p.set(PFlags::Z, result == 0);
            self.p.set(PFlags::C, (result > self.a) || full_wraparound);

            result
        };
    }

    fn cmp(&mut self, data_mode: DataMode) {
        let reg = self.a;
        self.compare(data_mode, reg, PFlags::M)
    }

    fn cpx(&mut self, data_mode: DataMode) {
        let reg = self.x;
        self.compare(data_mode, reg, PFlags::X)
    }

    fn cpy(&mut self, data_mode: DataMode) {
        let reg = self.y;
        self.compare(data_mode, reg, PFlags::X)
    }

    fn dec(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let result = op.wrapping_sub(1);

        let data = self.set_nz(result, PFlags::M);

        self.write_op(data, write_mode, PFlags::M);
    }

    fn dex(&mut self) {
        let result = self.x.wrapping_sub(1);

        self.x = self.set_nz(result, PFlags::X);
    }

    fn dey(&mut self) {
        let result = self.y.wrapping_sub(1);

        self.y = self.set_nz(result, PFlags::X);
    }

    fn inc(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let result = op.wrapping_add(1);

        let data = self.set_nz(result, PFlags::M);

        self.write_op(data, write_mode, PFlags::M);
    }

    fn inx(&mut self) {
        let result = self.x.wrapping_add(1);

        self.x = self.set_nz(result, PFlags::X);
    }

    fn iny(&mut self) {
        let result = self.y.wrapping_add(1);

        self.y = self.set_nz(result, PFlags::X);
    }

    fn and(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, PFlags::M);
        let result = self.a & op;

        self.a = self.set_nz(result, PFlags::M);
    }

    fn eor(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, PFlags::M);
        let result = self.a ^ op;

        self.a = self.set_nz(result, PFlags::M);
    }

    fn ora(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, PFlags::M);
        let result = self.a | op;

        self.a = self.set_nz(result, PFlags::M);
    }

    fn bit(&mut self, data_mode: DataMode) {
        let op = self.read_op(data_mode, PFlags::M);
        let result = self.a & op;

        if self.p.contains(PFlags::M) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::Z, result8 == 0);

            match data_mode {
                DataMode::Imm => {},
                _ => {
                    self.p.set(PFlags::N, (op & bit!(7, u16)) != 0);
                    self.p.set(PFlags::V, (op & bit!(6, u16)) != 0);
                }
            }
        } else {
            self.p.set(PFlags::Z, result == 0);

            match data_mode {
                DataMode::Imm => {},
                _ => {
                    self.p.set(PFlags::N, (op & bit!(15, u16)) != 0);
                    self.p.set(PFlags::V, (op & bit!(14, u16)) != 0);
                }
            }
        }
    }

    fn trb(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let result = self.a & op;

        self.set_z(result);

        let write_data = op & (!self.a);
        self.write_op(write_data, write_mode, PFlags::M);
    }

    fn tsb(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let result = self.a & op;

        self.set_z(result);

        let write_data = op | self.a;
        self.write_op(write_data, write_mode, PFlags::M);
    }

    fn asl(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let result = op << 1;

        self.p.set(PFlags::C, if self.p.contains(PFlags::M) {
            (op & bit!(7, u16)) != 0
        } else {
            (op & bit!(15, u16)) != 0
        });

        let write_data = self.set_nz(result, PFlags::M);
        self.write_op(write_data, write_mode, PFlags::M);
    }

    fn lsr(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let result = op >> 1;

        self.p.set(PFlags::C, (op & bit!(0, u16)) != 0);

        let write_data = self.set_nz(result, PFlags::M);
        self.write_op(write_data, write_mode, PFlags::M);
    }

    fn rol(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let result = (op << 1) | ((self.p & PFlags::C).bits() as u16);

        self.p.set(PFlags::C, if self.p.contains(PFlags::M) {
            (op & bit!(7, u16)) != 0
        } else {
            (op & bit!(15, u16)) != 0
        });

        let write_data = self.set_nz(result, PFlags::M);
        self.write_op(write_data, write_mode, PFlags::M);
    }

    fn ror(&mut self, data_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr_mode(data_mode, PFlags::M);
        let carry = ((self.p & PFlags::C).bits() as u16) << (if self.p.contains(PFlags::M) {7} else {15});
        let result = (op >> 1) | carry;

        self.p.set(PFlags::C, (op & bit!(0, u16)) != 0);

        let write_data = self.set_nz(result, PFlags::M);
        self.write_op(write_data, write_mode, PFlags::M);
    }
}

// Internal: Data and Flag setting Micro-ops
impl CPU {
    // Set N if high bit is 1, set Z if result is zero. Return 8 or 16 bit result.
    fn set_nz(&mut self, result: u16, flag_check: PFlags) -> u16 {
        if self.p.contains(flag_check) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);

            result
        }
    }

    // Set Z if result is zero. Always checks M flag.
    fn set_z(&mut self, result: u16) {
        if self.p.contains(PFlags::M) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::Z, result8 == 0);
        } else {
            self.p.set(PFlags::Z, result == 0);
        }
    }

    // Compare register with operand, and set flags accordingly.
    fn compare(&mut self, data_mode: DataMode, reg: u16, flag_check: PFlags) {
        let op = self.read_op(data_mode, flag_check);
        let result = reg.wrapping_sub(op);

        if self.p.contains(flag_check) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);
            self.p.set(PFlags::C, result8 >= reg);
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);
            self.p.set(PFlags::C, result >= reg);
        }
    }
}

// Internal: Memory and Addressing Micro-ops
impl CPU {
    // Read a byte from the (data) bus.
    fn read_data(&self, addr: u32) -> u8 {
        self.mem.read(addr)
    }

    // Write a byte to the (data) bus.
    fn write_data(&mut self, addr: u32, data: u8) {
        self.mem.write(addr, data);
    }

    // Read one or two bytes (based on the value of the M or X flag).
    fn read_addr(&self, addr: Addr, flag_check: PFlags) -> u16 {
        use self::Addr::*;

        match addr {
            Full(a) => {
                let data_lo = self.read_data(a);

                let data_hi = if !self.pe.contains(flag_check) {
                    self.read_data(a.wrapping_add(1))
                } else { 0 };
                
                make16!(data_hi, data_lo)
            },
            ZeroBank(a) => {
                let addr_lo = make24!(0, a);
                let data_lo = self.read_data(addr_lo);

                let data_hi = if !self.pe.contains(flag_check) {
                    let addr_hi = make24!(0, a.wrapping_add(1));
                    self.read_data(addr_hi)
                } else { 0 };
                
                make16!(data_hi, data_lo)
            }
        }
    }

    // Write one or two bytes (based on the value of the M or X flag).
    fn write_addr(&mut self, data: u16, addr: Addr, flag_check: PFlags) {
        use self::Addr::*;

        match addr {
            Full(a) => {
                self.write_data(a, lo!(data));

                if !self.pe.contains(flag_check) {
                    self.write_data(a.wrapping_add(1), hi!(data));
                }
            },
            ZeroBank(a) => {
                let addr_lo = make24!(0, a);
                self.write_data(addr_lo, lo!(data));

                if !self.pe.contains(flag_check) {
                    let addr_hi = make24!(0, a.wrapping_add(1));
                    self.write_data(addr_hi, hi!(data));
                }
            }
        }
    }

    // Fetch a byte from the PC.
    fn fetch(&mut self) -> u8 {
        // Read mem
        let data = self.read_data(make24!(self.pb, self.pc));
        self.pc = self.pc.wrapping_add(1);
        data
    }

    // Get an operand using the specified data mode.
    fn read_op(&mut self, data_mode: DataMode, flag_check: PFlags) -> u16 {
        use self::DataMode::*;

        match data_mode {
            Imm => self.immediate(flag_check),
            Acc => self.a,
            Mode(m) => {
                let addr = self.get_addr(m);
                self.read_addr(addr, flag_check)
            },
            Known(_) => unreachable!() // In practice we never read from known addresses.
        }
    }

    // Get an operand using the specified data mode and return the address if an addressing mode was used.
    fn read_op_and_addr_mode(&mut self, data_mode: DataMode, flag_check: PFlags) -> (u16, DataMode) {
        use self::DataMode::*;

        match data_mode {
            Imm => (self.immediate(flag_check), Imm),   // TODO: is this ever used?
            Acc => (self.a, Acc),
            Mode(m) => {
                let addr = self.get_addr(m);
                (self.read_addr(addr, flag_check), Known(addr))
            },
            Known(_) => unreachable!() // In practice we never read from known addresses.
        }
    }

    // Set an operand using the specified addressing mode.
    fn write_op(&mut self, data: u16, data_mode: DataMode, flag_check: PFlags) {
        use self::DataMode::*;

        match data_mode {
            Imm => unreachable!(),  // We can't write to immediate data.
            Acc => self.a = data,
            Mode(m) => {
                let addr = self.get_addr(m);
                self.write_addr(data, addr, flag_check);
            },
            Known(a) => self.write_addr(data, a, flag_check)
        }
    }

    // Get an address using the specified addressing mode.
    fn get_addr(&mut self, addr_mode: AddrMode) -> Addr {
        use self::AddrMode::*;

        match addr_mode {
            Abs             => self.absolute(),
            AbsX            => self.absolute_x(),
            AbsY            => self.absolute_y(),

            Dir             => self.direct(),
            DirX            => self.direct_x(),
            DirY            => self.direct_y(),
            DirPtrDBR       => self.direct_ptr_dbr(),
            DirPtrXDBR      => self.direct_ptr_x_dbr(),
            DirPtrDBRY      => self.direct_ptr_dbr_y(),
            DirPtr          => self.direct_ptr(),
            DirPtrY         => self.direct_ptr_y(),
            
            Long            => self.long(),
            LongX           => self.long_x(),
            Stack           => self.stack(),
            StackPtrDBRY    => self.stack_ptr_dbr_y()
        }
    }

    // Addressing modes:

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

        Addr::Full(make24!(self.db, imm_hi, imm_lo).wrapping_add(self.x as u32))
    }

    // $vvvv, Y
    fn absolute_y(&mut self) -> Addr {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        Addr::Full(make24!(self.db, imm_hi, imm_lo).wrapping_add(self.y as u32))
    }

    // $vv
    fn direct(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        /*let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            set_hi!(self.dp, imm)
        } else {
            self.dp.wrapping_add(imm as u16)
        };*/
        Addr::ZeroBank(self.dp.wrapping_add(imm))
    }

    // $vv, X
    fn direct_x(&mut self) -> Addr {
        let imm = self.fetch();

        let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            let addr_lo = (self.x as u8).wrapping_add(imm);
            set_lo!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.x).wrapping_add(imm as u16)
        };

        Addr::ZeroBank(addr)
    }

    // $vv, Y
    fn direct_y(&mut self) -> Addr {
        let imm = self.fetch();

        let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            let addr_lo = (self.y as u8).wrapping_add(imm);
            set_lo!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.y).wrapping_add(imm as u16)
        };

        Addr::ZeroBank(addr)
    }

    // ($vv)
    fn direct_ptr_dbr(&mut self) -> Addr {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            (set_lo!(self.dp, imm), set_lo!(self.dp, imm.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        Addr::Full(make24!(self.db, addr_hi, addr_lo))
    }

    // ($vv, X)
    fn direct_ptr_x_dbr(&mut self) -> Addr {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            let ptr_addr_lo = (self.x as u8).wrapping_add(imm);
            (set_lo!(self.dp, ptr_addr_lo), set_lo!(self.dp, ptr_addr_lo.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(self.x).wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        Addr::Full(make24!(self.db, addr_hi, addr_lo))
    }

    // ($vv), Y
    fn direct_ptr_dbr_y(&mut self) -> Addr {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            (set_lo!(self.dp, imm), set_lo!(self.dp, imm.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        Addr::Full(make24!(self.db, addr_hi, addr_lo).wrapping_add(self.y as u32))
    }

    // [$vv]
    fn direct_ptr(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        let ptr = self.dp.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_mid = make24!(0, ptr.wrapping_add(1));
        let ptr_hi = make24!(0, ptr.wrapping_add(2));

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

        let addr_lo = self.read_data(ptr_lo);
        let addr_mid = self.read_data(ptr_mid);
        let addr_hi = self.read_data(ptr_hi);

        Addr::Full(make24!(addr_hi, addr_mid, addr_lo).wrapping_add(self.y as u32))
    }

    // $vv, s
    fn stack(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        let addr = self.s.wrapping_add(imm);

        Addr::ZeroBank(addr)
    }

    // ($vv, s), Y
    fn stack_ptr_dbr_y(&mut self) -> Addr {
        let imm = self.fetch() as u16;

        let ptr = self.s.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_hi = make24!(0, ptr.wrapping_add(1));

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

    // #$vvvv
    fn immediate(&mut self, flag_check: PFlags) -> u16 {
        let imm_lo = self.fetch();

        let imm_hi = if !self.p.contains(flag_check) {
            self.fetch()
        } else { 0 };

        make16!(imm_hi, imm_lo)
    }
}