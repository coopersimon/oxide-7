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

    Imm(FlagCheck),
    Long,
    LongX,
    Stack,
    StackPtrDBRY
}

// Addressing mode flag check for immediate.
// TODO can I just use PFlags here instead?
#[derive(Clone, Copy)]
enum FlagCheck {
    M,
    X
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
            0x61 => self.adc(AddrMode::DirPtrXDBR),
            0x63 => self.adc(AddrMode::Stack),
            0x65 => self.adc(AddrMode::Dir),
            0x67 => self.adc(AddrMode::DirPtr),
            0x69 => self.adc(AddrMode::Imm(FlagCheck::M)),
            0x6D => self.adc(AddrMode::Abs),
            0x6F => self.adc(AddrMode::Long),
            0x71 => self.adc(AddrMode::DirPtrDBRY),
            0x72 => self.adc(AddrMode::DirPtrDBR),
            0x73 => self.adc(AddrMode::StackPtrDBRY),
            0x75 => self.adc(AddrMode::DirX),
            0x77 => self.adc(AddrMode::DirPtrY),
            0x79 => self.adc(AddrMode::AbsY),
            0x7D => self.adc(AddrMode::AbsX),
            0x7F => self.adc(AddrMode::LongX),
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

    // Read one or two bytes (based on the value of the M flag).
    fn m_read_data(&self, addr: u32) -> u16 {
        let data_lo = self.read_data(addr);

        let data_hi = if !self.pe.contains(PFlags::M) {
            self.read_data(addr.wrapping_add(1))
        } else { 0 };
        
        make16!(data_hi, data_lo)
    }

    // Read one or two bytes (based on the value of the M flag) from the zero bank.
    fn zero_m_read_data(&self, addr: u16) -> u16 {
        let addr_lo = make24!(0, addr);
        let data_lo = self.read_data(addr_lo);

        let data_hi = if !self.pe.contains(PFlags::M) {
            let addr_hi = make24!(0, addr.wrapping_add(1));
            self.read_data(addr_hi)
        } else { 0 };
        
        make16!(data_hi, data_lo)
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
            
            Imm(f)          => self.immediate(f),
            Long            => self.long(),
            LongX           => self.long_x(),
            Stack           => self.stack(),
            StackPtrDBRY    => self.stack_ptr_dbr_y(),
        }
    }

    // Addressing modes:

    // $vvvv
    fn absolute(&mut self) -> u16 {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let addr = make24!(self.db, imm_hi, imm_lo);

        self.m_read_data(addr)
    }

    // $vvvv, X
    fn absolute_x(&mut self) -> u16 {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let addr = make24!(self.db, imm_hi, imm_lo).wrapping_add(self.x as u32);

        self.m_read_data(addr)
    }

    // $vvvv, Y
    fn absolute_y(&mut self) -> u16 {
        let imm_lo = self.fetch();
        let imm_hi = self.fetch();

        let addr = make24!(self.db, imm_hi, imm_lo).wrapping_add(self.y as u32);

        self.m_read_data(addr)
    }

    // $vv
    fn direct(&mut self) -> u16 {
        let imm = self.fetch() as u16;

        /*let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            set_hi!(self.dp, imm)
        } else {
            self.dp.wrapping_add(imm as u16)
        };*/
        let addr = self.dp.wrapping_add(imm);

        self.zero_m_read_data(addr)
    }

    // $vv, X
    fn direct_x(&mut self) -> u16 {
        let imm = self.fetch();

        let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            let addr_lo = (self.x as u8).wrapping_add(imm);
            set_lo!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.x).wrapping_add(imm as u16)
        };

        self.zero_m_read_data(addr)
    }

    // $vv, Y
    fn direct_y(&mut self) -> u16 {
        let imm = self.fetch();

        let addr = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            let addr_lo = (self.y as u8).wrapping_add(imm);
            set_lo!(self.dp, addr_lo)
        } else {
            self.dp.wrapping_add(self.y).wrapping_add(imm as u16)
        };

        self.zero_m_read_data(addr)
    }

    // ($vv)
    fn direct_ptr_dbr(&mut self) -> u16 {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            (set_lo!(self.dp, imm), set_lo!(self.dp, imm.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        let addr = make24!(self.db, addr_hi, addr_lo);

        self.m_read_data(addr)
    }

    // ($vv, X)
    fn direct_ptr_x_dbr(&mut self) -> u16 {
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

        let addr = make24!(self.db, addr_hi, addr_lo);

        self.m_read_data(addr)
    }

    // ($vv), Y
    fn direct_ptr_dbr_y(&mut self) -> u16 {
        let imm = self.fetch();

        let (ptr_lo, ptr_hi) = if self.pe.contains(PFlags::E) && (lo!(self.dp) == 0) {
            (set_lo!(self.dp, imm), set_lo!(self.dp, imm.wrapping_add(1)))
        } else {
            let ptr_lo = self.dp.wrapping_add(imm as u16);
            (ptr_lo, ptr_lo.wrapping_add(1))
        };

        let addr_lo = self.read_data(make24!(0, ptr_lo));
        let addr_hi = self.read_data(make24!(0, ptr_hi));

        let addr = make24!(self.db, addr_hi, addr_lo).wrapping_add(self.y as u32);

        self.m_read_data(addr)
    }

    // [$vv]
    fn direct_ptr(&mut self) -> u16 {
        let imm = self.fetch() as u16;

        let ptr = self.dp.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_mid = make24!(0, ptr.wrapping_add(1));
        let ptr_hi = make24!(0, ptr.wrapping_add(2));

        let addr_lo = self.read_data(ptr_lo);
        let addr_mid = self.read_data(ptr_mid);
        let addr_hi = self.read_data(ptr_hi);

        let addr = make24!(addr_hi, addr_mid, addr_lo);

        self.m_read_data(addr)
    }

    // [$vv], Y
    fn direct_ptr_y(&mut self) -> u16 {
        let imm = self.fetch() as u16;

        let ptr = self.dp.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_mid = make24!(0, ptr.wrapping_add(1));
        let ptr_hi = make24!(0, ptr.wrapping_add(2));

        let addr_lo = self.read_data(ptr_lo);
        let addr_mid = self.read_data(ptr_mid);
        let addr_hi = self.read_data(ptr_hi);

        let addr = make24!(addr_hi, addr_mid, addr_lo).wrapping_add(self.y);

        self.m_read_data(addr)
    }

    // $vv, s
    fn stack(&mut self) -> u16 {
        let imm = self.fetch() as u16;

        let addr = self.s.wrapping_add(imm);

        self.zero_m_read_data(addr)
    }

    // ($vv, s), Y
    fn stack_ptr_dbr_y(&mut self) -> u16 {
        let imm = self.fetch() as u16;

        let ptr = self.s.wrapping_add(imm);

        let ptr_lo = make24!(0, ptr);
        let ptr_hi = make24!(0, ptr.wrapping_add(1));

        let addr_lo = self.read_data(ptr_lo);
        let addr_hi = self.read_data(ptr_hi);

        let addr = make24!(self.db, addr_hi, addr_lo).wrapping_add(self.y as u32);

        self.m_read_data(addr)
    }

    // #$vvvv
    fn immediate(&mut self, flag_check: FlagCheck) -> u16 {
        let imm_lo = self.fetch();

        let imm_hi = match flag_check {
            FlagCheck::M if self.p.contains(PFlags::M) => self.fetch(),
            FlagCheck::X if self.p.contains(PFlags::X) => self.fetch(),
            _ => 0
        };

        make16!(imm_hi, imm_lo)
    }

    // $vvvvvv
    fn long(&mut self) -> u16 {
        let imm_lo = self.fetch();
        let imm_mid = self.fetch();
        let imm_hi = self.fetch();

        let addr = make24!(imm_hi, imm_mid, imm_lo);

        self.m_read_data(addr)
    }

    // $vvvvvv, X
    fn long_x(&mut self) -> u16 {
        let imm_lo = self.fetch();
        let imm_mid = self.fetch();
        let imm_hi = self.fetch();

        let addr = make24!(imm_hi, imm_mid, imm_lo).wrapping_add(self.x as u32);

        self.m_read_data(addr)
    }
}