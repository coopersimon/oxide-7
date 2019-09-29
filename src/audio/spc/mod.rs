// SPC-700 Audio processor
mod mem;

use bitflags::bitflags;

use mem::SPCBus;

bitflags! {
    #[derive(Default)]
    struct PSFlags: u8 {
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

enum DataMode {
    Imm,            // Immediate data
    Acc,            // Accumulator register data
    X,              // X register data
    Y,              // Y register data
    Mode(AddrMode), // Data in memory
    Known(u16)      // Data in memory with known address
}

enum AddrMode {
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

const SPC_OP: usize = 1;   // Number of cycles for an internal op.

pub struct SPC {
    a:      u8,         // Accumulator
    x:      u8,         // X-Index
    y:      u8,         // Y-Index
    sp:     u8,         // Stack Pointer
    pc:     u16,        // Program Counter

    ps:     PSFlags,    // Program Status Word

    bus:    SPCBus
}

impl SPC {
    pub fn new() -> Self {
        SPC {
            a:      0,
            x:      0,
            y:      0,
            sp:     0xEF,
            pc:     0xFFC0,

            ps:     PSFlags::Z,

            bus:    SPCBus::new()
        }
    }

    pub fn step(&mut self) {

    }
}

// Internal
impl SPC {
    fn execute_instruction(&mut self) {
        use DataMode::*;
        use AddrMode::*;

        let instr = self.fetch();

        match instr {
            0x99 => self.adc(Mode(XIndir), Mode(YIndir)),
            0x88 => self.adc(Acc, Imm),
            0x86 => self.adc(Acc, Mode(XIndir)),
            0x97 => self.adc(Acc, Mode(DirPtrY)),
            0x87 => self.adc(Acc, Mode(DirXPtr)),
            0x84 => self.adc(Acc, Mode(Dir)),
            0x94 => self.adc(Acc, Mode(DirX)),
            0x85 => self.adc(Acc, Mode(Abs)),
            0x95 => self.adc(Acc, Mode(AbsX)),
            0x96 => self.adc(Acc, Mode(AbsY)),
            0x89 => self.adc(Mode(Dir), Mode(Dir)),
            0x98 => self.adc(Mode(Dir), Imm),

            0x7A => self.addw(),

            0xB9 => self.sbc(Mode(XIndir), Mode(YIndir)),
            0xA8 => self.sbc(Acc, Imm),
            0xA6 => self.sbc(Acc, Mode(XIndir)),
            0xB7 => self.sbc(Acc, Mode(DirPtrY)),
            0xA7 => self.sbc(Acc, Mode(DirXPtr)),
            0xA4 => self.sbc(Acc, Mode(Dir)),
            0xB4 => self.sbc(Acc, Mode(DirX)),
            0xA5 => self.sbc(Acc, Mode(Abs)),
            0xB5 => self.sbc(Acc, Mode(AbsX)),
            0xB6 => self.sbc(Acc, Mode(AbsY)),
            0xA9 => self.sbc(Mode(Dir), Mode(Dir)),
            0xB8 => self.sbc(Mode(Dir), Imm),

            0x9A => self.subw(),

            0xBC => self.inc(Acc),
            0xAB => self.inc(Mode(Dir)),
            0xBB => self.inc(Mode(DirX)),
            0xAC => self.inc(Mode(Abs)),
            0x3D => self.inc(X),
            0xFC => self.inc(Y),

            0x3A => self.incw(),

            0x9C => self.dec(Acc),
            0x8B => self.dec(Mode(Dir)),
            0x9B => self.dec(Mode(DirX)),
            0x8C => self.dec(Mode(Abs)),
            0x1D => self.dec(X),
            0xDC => self.dec(Y),

            0x1A => self.decw(),

            0xCF => self.mul(),
            0x9E => self.div(),

            0x39 => self.and(Mode(XIndir), Mode(YIndir)),
            0x28 => self.and(Acc, Imm),
            0x26 => self.and(Acc, Mode(XIndir)),
            0x37 => self.and(Acc, Mode(DirPtrY)),
            0x27 => self.and(Acc, Mode(DirXPtr)),
            0x24 => self.and(Acc, Mode(Dir)),
            0x34 => self.and(Acc, Mode(DirX)),
            0x25 => self.and(Acc, Mode(Abs)),
            0x35 => self.and(Acc, Mode(AbsX)),
            0x36 => self.and(Acc, Mode(AbsY)),
            0x29 => self.and(Mode(Dir), Mode(Dir)),
            0x38 => self.and(Mode(Dir), Imm),

            0x59 => self.eor(Mode(XIndir), Mode(YIndir)),
            0x48 => self.eor(Acc, Imm),
            0x46 => self.eor(Acc, Mode(XIndir)),
            0x57 => self.eor(Acc, Mode(DirPtrY)),
            0x47 => self.eor(Acc, Mode(DirXPtr)),
            0x44 => self.eor(Acc, Mode(Dir)),
            0x54 => self.eor(Acc, Mode(DirX)),
            0x45 => self.eor(Acc, Mode(Abs)),
            0x55 => self.eor(Acc, Mode(AbsX)),
            0x56 => self.eor(Acc, Mode(AbsY)),
            0x49 => self.eor(Mode(Dir), Mode(Dir)),
            0x58 => self.eor(Mode(Dir), Imm),

            0x19 => self.or(Mode(XIndir), Mode(YIndir)),
            0x08 => self.or(Acc, Imm),
            0x06 => self.or(Acc, Mode(XIndir)),
            0x17 => self.or(Acc, Mode(DirPtrY)),
            0x07 => self.or(Acc, Mode(DirXPtr)),
            0x04 => self.or(Acc, Mode(Dir)),
            0x14 => self.or(Acc, Mode(DirX)),
            0x05 => self.or(Acc, Mode(Abs)),
            0x15 => self.or(Acc, Mode(AbsX)),
            0x16 => self.or(Acc, Mode(AbsY)),
            0x09 => self.or(Mode(Dir), Mode(Dir)),
            0x18 => self.or(Mode(Dir), Imm),

            0x1C => self.asl(Acc),
            0x0B => self.asl(Mode(Dir)),
            0x1B => self.asl(Mode(DirX)),
            0x0C => self.asl(Mode(Abs)),

            0x5C => self.lsr(Acc),
            0x4B => self.lsr(Mode(Dir)),
            0x5B => self.lsr(Mode(DirX)),
            0x4C => self.lsr(Mode(Abs)),

            0x3C => self.rol(Acc),
            0x2B => self.rol(Mode(Dir)),
            0x3B => self.rol(Mode(DirX)),
            0x2C => self.rol(Mode(Abs)),

            0x7C => self.ror(Acc),
            0x6B => self.ror(Mode(Dir)),
            0x7B => self.ror(Mode(DirX)),
            0x6C => self.ror(Mode(Abs)),

            0x02 => self.set1(0),
            0x22 => self.set1(1),
            0x42 => self.set1(2),
            0x62 => self.set1(3),
            0x82 => self.set1(4),
            0xA2 => self.set1(5),
            0xC2 => self.set1(6),
            0xE2 => self.set1(7),

            0x12 => self.clr1(0),
            0x32 => self.clr1(1),
            0x52 => self.clr1(2),
            0x72 => self.clr1(3),
            0x92 => self.clr1(4),
            0xB2 => self.clr1(5),
            0xD2 => self.clr1(6),
            0xF2 => self.clr1(7),

            0x6A => self.and1(true),
            0x4A => self.and1(false),
            0x8A => self.eor1(),
            0x2A => self.or1(true),
            0x0A => self.or1(false),

            0x79 => self.cmp(Mode(XIndir), Mode(YIndir)),
            0x68 => self.cmp(Acc, Imm),
            0x66 => self.cmp(Acc, Mode(XIndir)),
            0x77 => self.cmp(Acc, Mode(DirPtrY)),
            0x67 => self.cmp(Acc, Mode(DirXPtr)),
            0x64 => self.cmp(Acc, Mode(Dir)),
            0x74 => self.cmp(Acc, Mode(DirX)),
            0x65 => self.cmp(Acc, Mode(Abs)),
            0x75 => self.cmp(Acc, Mode(AbsX)),
            0x76 => self.cmp(Acc, Mode(AbsY)),
            0x69 => self.cmp(Mode(Dir), Mode(Dir)),
            0x78 => self.cmp(Mode(Dir), Imm),

            0xC8 => self.cmp(X, Imm),
            0x3E => self.cmp(X, Mode(Dir)),
            0x1E => self.cmp(X, Mode(Abs)),

            0xAD => self.cmp(Y, Imm),
            0x7E => self.cmp(Y, Mode(Dir)),
            0x5E => self.cmp(Y, Mode(Abs)),

            0x5A => self.cmpw(),

            0xED => self.notc(),

            0x80 => self.set_flag(PSFlags::C),  // SETC
            0x40 => self.set_flag(PSFlags::P),  // SETP
            0xA0 => self.set_flag(PSFlags::I),  // EI
            
            0x60 => self.clear_flag(PSFlags::C),    // CLRC
            0x20 => self.clear_flag(PSFlags::P),    // CLRP
            0xE0 => self.clear_flag(PSFlags::V | PSFlags::H),   // CLRV
            0xC0 => self.clear_flag(PSFlags::I),    // DI TODO: is this the right way around?

            0xE8 => self.mov_set_flags(Acc, Imm),
            0xE6 => self.mov_set_flags(Acc, Mode(XIndir)),
            0xBF => self.mov_set_flags(Acc, Mode(XIndirInc)),
            0xF7 => self.mov_set_flags(Acc, Mode(DirPtrY)),
            0xE7 => self.mov_set_flags(Acc, Mode(DirXPtr)),
            0x7D => self.mov_set_flags(Acc, X),
            0xDD => self.mov_set_flags(Acc, Y),
            0xE4 => self.mov_set_flags(Acc, Mode(Dir)),
            0xF4 => self.mov_set_flags(Acc, Mode(DirX)),
            0xE5 => self.mov_set_flags(Acc, Mode(Abs)),
            0xF5 => self.mov_set_flags(Acc, Mode(AbsX)),
            0xF6 => self.mov_set_flags(Acc, Mode(AbsY)),

            0xBD => self.mov_sp_x(),
            0x9D => self.mov_x_sp(),

            0xCD => self.mov_set_flags(X, Imm),
            0x5D => self.mov_set_flags(X, Acc),
            0xF8 => self.mov_set_flags(X, Mode(Dir)),
            0xF9 => self.mov_set_flags(X, Mode(DirY)),
            0xE9 => self.mov_set_flags(X, Mode(Abs)),

            0x8D => self.mov_set_flags(Y, Imm),
            0xFD => self.mov_set_flags(Y, Acc),
            0xEB => self.mov_set_flags(Y, Mode(Dir)),
            0xFB => self.mov_set_flags(Y, Mode(DirX)),
            0xEC => self.mov_set_flags(Y, Mode(Abs)),

            0xAF => self.mov_no_flags(Mode(XIndirInc), Acc),
            0xC6 => self.mov_no_flags(Mode(XIndir), Acc),
            0xD7 => self.mov_no_flags(Mode(DirPtrY), Acc),
            0xC7 => self.mov_no_flags(Mode(DirXPtr), Acc),

            0xFA => self.mov_no_flags(Mode(Dir), Mode(Dir)),

            0xD4 => self.mov_no_flags(Mode(DirX), Acc),
            0xD9 => self.mov_no_flags(Mode(DirY), X),
            0xDB => self.mov_no_flags(Mode(DirX), Y),

            0x8F => self.mov_no_flags(Mode(Dir), Imm),
            0xC4 => self.mov_no_flags(Mode(Dir), Acc),
            0xD8 => self.mov_no_flags(Mode(Dir), X),
            0xCB => self.mov_no_flags(Mode(Dir), Y),

            0xD5 => self.mov_no_flags(Mode(AbsX), Acc),
            0xD6 => self.mov_no_flags(Mode(AbsY), Acc),
            0xC5 => self.mov_no_flags(Mode(Abs), Acc),
            0xC9 => self.mov_no_flags(Mode(Abs), X),
            0xCC => self.mov_no_flags(Mode(Abs), Y),

            0x9F => self.xcn(),

            0x00 => self.nop(),
        }
    }

    fn clock_inc(&mut self, _cycles: usize) {

    }

    fn fetch(&mut self) -> u8 {
        let data = self.read_data(self.pc);
        self.pc = self.pc.wrapping_add(1);
        data
    }
}

// Instructions: Arithmetic
impl SPC {
    // op1 = op1 + op2 + C
    fn adc(&mut self, op1_mode: DataMode, op2_mode: DataMode) {
        let (op1, op1_addr) = self.read_op_and_addr(op1_mode);
        let op2 = self.read_op(op2_mode);

        let result = op1.wrapping_add(op2).wrapping_add(self.carry());

        let full_wraparound = (result == op1) && (op2 != 0);
        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::V, ((result as i8) < (op1 as i8)) || full_wraparound);
        self.ps.set(PSFlags::H, true);  // TODO
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, (result < op1) || full_wraparound);

        self.write_op(op1_addr, result);
    }

    // op1 = op1 + op2 + C
    fn sbc(&mut self, op1_mode: DataMode, op2_mode: DataMode) {
        let (op1, op1_addr) = self.read_op_and_addr(op1_mode);
        let op2 = self.read_op(op2_mode);

        let result = op1.wrapping_sub(op2).wrapping_sub(1).wrapping_add(self.carry());

        let full_wraparound = (result == op1) && (op2 != 0);
        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::V, ((result as i8) > (op1 as i8)) || full_wraparound);
        self.ps.set(PSFlags::H, true);  // TODO
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, (result > op1) || full_wraparound);

        self.write_op(op1_addr, result);
    }

    // 16-bit add
    fn addw(&mut self) {
        let ya = make16!(self.y, self.a);

        let result = ya.wrapping_add(self.read_op_16().0);

        self.ps.set(PSFlags::N, test_bit!(result, 15));
        self.ps.set(PSFlags::V, (result as i16) < (ya as i16));
        self.ps.set(PSFlags::H, true);  // TODO
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, result < ya);

        self.y = hi!(result);
        self.a = lo!(result);
    }

    // 16-bit sub
    fn subw(&mut self) {
        let ya = make16!(self.y, self.a);

        let result = ya.wrapping_sub(self.read_op_16().0);

        self.ps.set(PSFlags::N, test_bit!(result, 15));
        self.ps.set(PSFlags::V, (result as i16) > (ya as i16));
        self.ps.set(PSFlags::H, true);  // TODO
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, result > ya);

        self.y = hi!(result);
        self.a = lo!(result);
    }

    fn mul(&mut self) {
        let result = (self.a as u16).wrapping_mul(self.y as u16);

        self.ps.set(PSFlags::N, test_bit!(result, 15));
        self.ps.set(PSFlags::Z, hi!(result) == 0);

        self.y = hi!(result);
        self.a = lo!(result);
    }

    fn div(&mut self) {
        let ya = make16!(self.y, self.a);
        let result = lo!(ya.wrapping_div(self.x as u16));
        let modulo = lo!(ya % (self.x as u16));

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, hi!(result) == 0);

        self.y = modulo;
        self.a = result;
    }

    fn inc(&mut self, op_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr(op_mode);

        let result = op.wrapping_add(1);

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);

        self.write_op(write_mode, result);
    }

    fn dec(&mut self, op_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr(op_mode);

        let result = op.wrapping_sub(1);

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);

        self.write_op(write_mode, result);
    }

    fn incw(&mut self) {
        let (op, op_addr) = self.read_op_16();

        let result = op.wrapping_add(1);

        self.ps.set(PSFlags::N, test_bit!(result, 15));
        self.ps.set(PSFlags::Z, result == 0);

        self.write_op_16(op_addr, result);
    }

    fn decw(&mut self) {
        let (op, op_addr) = self.read_op_16();

        let result = op.wrapping_sub(1);

        self.ps.set(PSFlags::N, test_bit!(result, 15));
        self.ps.set(PSFlags::Z, result == 0);

        self.write_op_16(op_addr, result);
    }
}

// Instructions: bitwise
impl SPC {
    fn and(&mut self, op1_mode: DataMode, op2_mode: DataMode) {
        let (op1, write_mode) = self.read_op_and_addr(op1_mode);
        let op2 = self.read_op(op2_mode);

        let result = op1 & op2;

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);

        self.write_op(write_mode, result);
    }

    fn eor(&mut self, op1_mode: DataMode, op2_mode: DataMode) {
        let (op1, write_mode) = self.read_op_and_addr(op1_mode);
        let op2 = self.read_op(op2_mode);

        let result = op1 ^ op2;

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);

        self.write_op(write_mode, result);
    }

    fn or(&mut self, op1_mode: DataMode, op2_mode: DataMode) {
        let (op1, write_mode) = self.read_op_and_addr(op1_mode);
        let op2 = self.read_op(op2_mode);

        let result = op1 | op2;

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);

        self.write_op(write_mode, result);
    }

    fn asl(&mut self, op_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr(op_mode);
        let result = op << 1;

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, test_bit!(op, 7, u8));

        self.write_op(write_mode, result);
    }

    fn lsr(&mut self, op_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr(op_mode);
        let result = op >> 1;

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, test_bit!(op, 0, u8));

        self.write_op(write_mode, result);
    }

    fn rol(&mut self, op_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr(op_mode);
        let result = (op << 1) | self.carry();

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, test_bit!(op, 7, u8));

        self.write_op(write_mode, result);
    }

    fn ror(&mut self, op_mode: DataMode) {
        let (op, write_mode) = self.read_op_and_addr(op_mode);
        let carry = self.carry() << 7;
        let result = (op >> 1) | carry;

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, test_bit!(op, 0, u8));

        self.write_op(write_mode, result);
    }

    fn set1(&mut self, bit_num: u8) {
        let bit = bit!(bit_num);
        let op_addr = self.direct();

        let data = self.read_data(op_addr) | bit;

        self.write_data(op_addr, data);
    }

    fn clr1(&mut self, bit_num: u8) {
        let mask = !bit!(bit_num);
        let op_addr = self.direct();

        let data = self.read_data(op_addr) & mask;

        self.write_data(op_addr, data);
    }
}

// Instructions: Flags
impl SPC {
    // C = C & m.b / C = C & !m.b
    fn and1(&mut self, not: bool) {
        let (addr, bit) = self.absolute_bit();
        let data = self.read_data(addr) >> bit;
        let op = (if not {!data} else {data}) & 1;

        self.ps.set(PSFlags::C, (self.ps & PSFlags::from_bits_truncate(op)) == PSFlags::C);
    }

    // C = C ^ m.b
    fn eor1(&mut self) {
        let (addr, bit) = self.absolute_bit();
        let data = self.read_data(addr) >> bit;
        let op = data & 1;

        self.ps.set(PSFlags::C, (self.ps & PSFlags::C) != PSFlags::from_bits_truncate(op));
    }

    // C = C | m.b / C = C | !m.b
    fn or1(&mut self, not: bool) {
        let (addr, bit) = self.absolute_bit();
        let data = self.read_data(addr) >> bit;
        let op = (if not {!data} else {data}) & 1;

        if op != 0 {
            self.ps.insert(PSFlags::C);
        }
    }

    // C = !C
    fn notc(&mut self) {
        self.ps.toggle(PSFlags::C);
    }

    fn set_flag(&mut self, flag: PSFlags) {
        self.ps.insert(flag);
    }

    fn clear_flag(&mut self, flag: PSFlags) {
        self.ps.remove(flag);
    }

    fn cmp(&mut self, op1_mode: DataMode, op2_mode: DataMode) {
        let op1 = self.read_op(op1_mode);
        let op2 = self.read_op(op2_mode);
        let result = op1.wrapping_sub(op2);

        self.ps.set(PSFlags::N, test_bit!(result, 7, u8));
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, op1 >= op2);
    }

    fn cmpw(&mut self) {
        let ya = make16!(self.y, self.a);
        let op = self.read_op_16().0;

        let result = ya.wrapping_sub(op);

        // Set flags
        self.ps.set(PSFlags::N, test_bit!(result, 15));
        self.ps.set(PSFlags::Z, result == 0);
        self.ps.set(PSFlags::C, ya >= op);
    }
}

// Instructions: moving data
impl SPC {
    // Load into register, and set flags.
    fn mov_set_flags(&mut self, dst_mode: DataMode, src_mode: DataMode) {
        let data = self.read_op(src_mode);

        self.ps.set(PSFlags::N, test_bit!(data, 7, u8));
        self.ps.set(PSFlags::Z, data == 0);

        self.write_op(dst_mode, data);
    }

    // Store into memory.
    fn mov_no_flags(&mut self, dst_mode: DataMode, src_mode: DataMode) {
        let data = self.read_op(src_mode);

        self.write_op(dst_mode, data);
    }

    // Move SP into X, and set flags.
    fn mov_x_sp(&mut self) {
        self.x = self.sp;

        self.ps.set(PSFlags::N, test_bit!(self.x, 7, u8));
        self.ps.set(PSFlags::Z, self.x == 0);
    }

    // Move X into SP.
    fn mov_sp_x(&mut self) {
        self.sp = self.x;
    }
}

// Instructions: misc
impl SPC {
    fn xcn(&mut self) {
        let a_lo = (self.a >> 4) & 0xF;
        let a_hi = (self.a & 0xF) << 4;

        self.a = a_lo | a_hi;

        self.ps.set(PSFlags::N, test_bit!(self.a, 7, u8));
        self.ps.set(PSFlags::Z, self.a == 0);
    }

    fn nop(&mut self) {
        self.clock_inc(SPC_OP);
    }
}

// Misc helper functions
impl SPC {
    #[inline]
    fn carry(&self) -> u8 {
        (self.ps & PSFlags::C).bits()
    }
}

// Internal data functions
impl SPC {
    // Read data from bus.
    fn read_data(&mut self, addr: u16) -> u8 {
        let data = self.bus.read(addr);
        self.clock_inc(SPC_OP);
        data
    }

    // Write data to bus.
    fn write_data(&mut self, addr: u16, data: u8) {
        self.bus.write(addr, data);
        self.clock_inc(SPC_OP);
    }

    // Get an operand using the specified data mode.
    fn read_op(&mut self, data_mode: DataMode) -> u8 {
        use DataMode::*;

        match data_mode {
            Imm => self.fetch(),
            Acc => self.a,
            X => self.x,
            Y => self.y,
            Mode(m) => {
                let addr = self.get_op_addr(m);
                self.read_data(addr)
            },
            Known(_) => unreachable!()  // We never read from known addresses.
        }
    }

    // Get an operand using the specified data mode and return the address if an addressing mode was used.
    fn read_op_and_addr(&mut self, data_mode: DataMode) -> (u8, DataMode) {
        use DataMode::*;

        match data_mode {
            Imm => unreachable!(),  // We never write back to immediate data.
            Acc => (self.a, Acc),
            X => (self.x, X),
            Y => (self.y, Y),
            Mode(m) => {
                let addr = self.get_op_addr(m);
                (self.read_data(addr), Known(addr))
            },
            Known(_) => unreachable!()  // We never read from known addresses.
        }
    }

    // Write an operand's data back.
    fn write_op(&mut self, data_mode: DataMode, data: u8) {
        use DataMode::*;

        match data_mode {
            Imm => unreachable!(),  // We never write back to immediate data.
            Acc => self.a = data,
            X => self.x = data,
            Y => self.y = data,
            Mode(m) => {
                let addr = self.get_op_addr(m);
                self.write_data(addr, data);
            },
            Known(a) => self.write_data(a, data)
        }
    }

    // Get 16-bit operand and address for 16-bit operations. Uses direct addressing.
    fn read_op_16(&mut self) -> (u16, u8) {
        let op_addr_lo = self.fetch();

        let op_lo = self.read_data(self.direct_page(op_addr_lo));
        let op_hi = self.read_data(self.direct_page(op_addr_lo.wrapping_add(1)));

        (make16!(op_hi, op_lo), op_addr_lo)
    }

    fn write_op_16(&mut self, addr_lo: u8, data: u16) {
        self.write_data(self.direct_page(addr_lo), lo!(data));
        self.write_data(self.direct_page(addr_lo.wrapping_add(1)), hi!(data));
    }

    // Get address of operand for addressing mode.
    fn get_op_addr(&mut self, addr_mode: AddrMode) -> u16 {
        use AddrMode::*;

        match addr_mode {
            XIndir      => self.x_indirect(),
            YIndir      => self.y_indirect(),
            XIndirInc   => self.x_indirect_inc(),

            Dir         => self.direct(),
            DirX        => self.direct_x(),
            DirY        => self.direct_y(),
            DirPtrY     => self.direct_ptr_y(),
            DirXPtr     => self.direct_x_ptr(),

            Abs         => self.absolute(),
            AbsX        => self.absoluteX(),
            AbsY        => self.absoluteY()
        }
    }
}

// Addressing modes
impl SPC {
    // Make 16-bit address using direct page as high byte
    fn direct_page(&self, addr_lo: u8) -> u16 {
        let addr_hi = if self.ps.contains(PSFlags::P) {1} else {0};

        make16!(addr_hi, addr_lo)
    }

    // (X)
    fn x_indirect(&self) -> u16 {
        self.direct_page(self.x)
    }

    // (Y)
    fn y_indirect(&self) -> u16 {
        self.direct_page(self.y)
    }

    // (X)+
    fn x_indirect_inc(&mut self) -> u16 {
        let addr = self.direct_page(self.x);
        self.x = self.x.wrapping_add(1);
        addr
    }

    // dp
    fn direct(&mut self) -> u16 {
        let addr_lo = self.fetch();

        self.direct_page(addr_lo)
    }

    // dp+X
    fn direct_x(&mut self) -> u16 {
        let addr_lo = self.fetch().wrapping_add(self.x);

        self.direct_page(addr_lo)
    }

    // dp+Y
    fn direct_y(&mut self) -> u16 {
        let addr_lo = self.fetch().wrapping_add(self.y);

        self.direct_page(addr_lo)
    }

    // !abs
    fn absolute(&mut self) -> u16 {
        let addr_lo = self.fetch();
        let addr_hi = self.fetch();

        make16!(addr_hi, addr_lo)
    }

    // !abs+X
    fn absoluteX(&mut self) -> u16 {
        let addr_lo = self.fetch();
        let addr_hi = self.fetch();

        make16!(addr_hi, addr_lo).wrapping_add(self.x as u16)
    }

    // !abs+Y
    fn absoluteY(&mut self) -> u16 {
        let addr_lo = self.fetch();
        let addr_hi = self.fetch();

        make16!(addr_hi, addr_lo).wrapping_add(self.y as u16)
    }

    // [dp+X]
    fn direct_x_ptr(&mut self) -> u16 {
        let addr = self.direct_x();
        
        let ptr_lo = self.read_data(addr);
        let ptr_hi = self.read_data(addr.wrapping_add(1));  // TODO: wrap in page?

        make16!(ptr_hi, ptr_lo)
    }

    // [dp]+Y
    fn direct_ptr_y(&mut self) -> u16 {
        let addr = self.direct();
        
        let ptr_lo = self.read_data(addr);
        let ptr_hi = self.read_data(addr.wrapping_add(1));  // TODO: wrap in page?

        make16!(ptr_hi, ptr_lo).wrapping_add(self.y as u16)
    }

    // m.b
    fn absolute_bit(&mut self) -> (u16, u8) {
        let abs = self.absolute();

        let addr = abs & 0x1FFF;
        let bit = (abs >> 13) as u8;

        (addr, bit)
    }
}