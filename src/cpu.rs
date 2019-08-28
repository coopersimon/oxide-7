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
    Unknown(AddrMode),  // Find the address using the given Addressing mode
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
            0x61 => self.adc(Unknown(DirPtrXDBR)),
            0x63 => self.adc(Unknown(Stack)),
            0x65 => self.adc(Unknown(Dir)),
            0x67 => self.adc(Unknown(DirPtr)),
            0x69 => self.adc(Imm),
            0x6D => self.adc(Unknown(Abs)),
            0x6F => self.adc(Unknown(Long)),
            0x71 => self.adc(Unknown(DirPtrDBRY)),
            0x72 => self.adc(Unknown(DirPtrDBR)),
            0x73 => self.adc(Unknown(StackPtrDBRY)),
            0x75 => self.adc(Unknown(DirX)),
            0x77 => self.adc(Unknown(DirPtrY)),
            0x79 => self.adc(Unknown(AbsY)),
            0x7D => self.adc(Unknown(AbsX)),
            0x7F => self.adc(Unknown(LongX)),

            0xE1 => self.sbc(Unknown(DirPtrXDBR)),
            0xE3 => self.sbc(Unknown(Stack)),
            0xE5 => self.sbc(Unknown(Dir)),
            0xE7 => self.sbc(Unknown(DirPtr)),
            0xE9 => self.sbc(Imm),
            0xED => self.sbc(Unknown(Abs)),
            0xEF => self.sbc(Unknown(Long)),
            0xF1 => self.sbc(Unknown(DirPtrDBRY)),
            0xF2 => self.sbc(Unknown(DirPtrDBR)),
            0xF3 => self.sbc(Unknown(StackPtrDBRY)),
            0xF5 => self.sbc(Unknown(DirX)),
            0xF7 => self.sbc(Unknown(DirPtrY)),
            0xF9 => self.sbc(Unknown(AbsY)),
            0xFD => self.sbc(Unknown(AbsX)),
            0xFF => self.sbc(Unknown(LongX)),

            0xC1 => self.cmp(Unknown(DirPtrXDBR)),
            0xC3 => self.cmp(Unknown(Stack)),
            0xC5 => self.cmp(Unknown(Dir)),
            0xC7 => self.cmp(Unknown(DirPtr)),
            0xC9 => self.cmp(Imm),
            0xCD => self.cmp(Unknown(Abs)),
            0xCF => self.cmp(Unknown(Long)),
            0xD1 => self.cmp(Unknown(DirPtrDBRY)),
            0xD2 => self.cmp(Unknown(DirPtrDBR)),
            0xD3 => self.cmp(Unknown(StackPtrDBRY)),
            0xD5 => self.cmp(Unknown(DirX)),
            0xD7 => self.cmp(Unknown(DirPtrY)),
            0xD9 => self.cmp(Unknown(AbsY)),
            0xDD => self.cmp(Unknown(AbsX)),
            0xDF => self.cmp(Unknown(LongX)),

            0xE0 => self.cpx(Imm),
            0xE4 => self.cpx(Unknown(Dir)),
            0xEC => self.cpx(Unknown(Abs)),
            0xC0 => self.cpy(Imm),
            0xC4 => self.cpy(Unknown(Dir)),
            0xCC => self.cpy(Unknown(Abs)),
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
        let (op, write_addr) = self.read_op_and_addr(data_mode, PFlags::M);
        let result = op.wrapping_sub(1);

        let data = if self.p.contains(PFlags::M) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);

            result
        };

        let write_data_mode = if let Some(addr) = write_addr {
            DataMode::Known(addr)
        } else {
            data_mode
        };
        self.write_op(data, write_data_mode, PFlags::M);
    }

    fn dex(&mut self) {
        let result = self.x.wrapping_sub(1);

        self.x = if self.p.contains(PFlags::X) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);

            result
        };
    }

    fn dey(&mut self) {
        let result = self.y.wrapping_sub(1);

        self.y = if self.p.contains(PFlags::X) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);

            result
        };
    }

    fn inc(&mut self, data_mode: DataMode) {
        let (op, write_addr) = self.read_op_and_addr(data_mode, PFlags::M);
        let result = op.wrapping_add(1);

        let data = if self.p.contains(PFlags::M) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);

            result
        };

        let write_data_mode = if let Some(addr) = write_addr {
            DataMode::Known(addr)
        } else {
            data_mode
        };
        self.write_op(data, write_data_mode, PFlags::M);
    }

    fn inx(&mut self) {
        let result = self.x.wrapping_add(1);

        self.x = if self.p.contains(PFlags::X) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);

            result
        };
    }

    fn iny(&mut self) {
        let result = self.y.wrapping_add(1);

        self.y = if self.p.contains(PFlags::X) {
            let result8 = result & 0xFF;
            self.p.set(PFlags::N, (result8 & bit!(7, u16)) != 0);
            self.p.set(PFlags::Z, result8 == 0);

            result8
        } else {
            self.p.set(PFlags::N, (result & bit!(15, u16)) != 0);
            self.p.set(PFlags::Z, result == 0);

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
            Unknown(m) => {
                let addr = self.get_addr(m);
                self.read_addr(addr, flag_check)
            },
            Known(_) => unreachable!() // In practice we never read from known addresses.
        }
    }

    // Get an operand using the specified data mode and return the address if an addressing mode was used.
    fn read_op_and_addr(&mut self, data_mode: DataMode, flag_check: PFlags) -> (u16, Option<Addr>) {
        use self::DataMode::*;

        match data_mode {
            Imm => (self.immediate(flag_check), None),
            Acc => (self.a, None),
            Unknown(m) => {
                let addr = self.get_addr(m);
                (self.read_addr(addr, flag_check), Some(addr))
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
            Unknown(m) => {
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