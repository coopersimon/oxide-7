// NEC uPD77C25 - Used as DSP
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct AccFlags: u8 {
        const S0 = bit!(5);     // Sign
        const Z = bit!(4);      // Zero
        const C = bit!(3);      // Carry
        const OV0 = bit!(2);    // Overflow
        const S1 = bit!(1);     // Direction of overflow
        const OV1 = bit!(0);    // Number of overflows
    }
}

impl AccFlags {
    fn set_sz(&mut self, result: u16) {
        // TODO: Should set s1 if not ovf instruction
        self.set(AccFlags::S0, test_bit!(result, 15));
        self.set(AccFlags::Z, result == 0);
    }

    fn set_add_ovf(&mut self, in_1: u16, in_2: u16, result: u16) {
        // If signs are the same, the result should have the same sign.
        // XNOR inputs to check they have the same sign.
        // XOR result and input to check they have different signs.
        // If both are set, then an overflow occurred.
        let set_ov0 = test_bit!(!(in_1 ^ in_2) & (in_1 ^ result), 15);
        self.set(AccFlags::OV0, set_ov0);
        if set_ov0 {
            self.set(AccFlags::S1, self.contains(AccFlags::S0));
            self.toggle(AccFlags::OV1);
        }
    }

    fn set_sub_ovf(&mut self, in_1: u16, in_2: u16, result: u16) {
        // If signs are different, the result should have the same sign as input 1.
        // XOR inputs to check they have different signs.
        // XOR result and input 1 to check they have different signs.
        // If both are set, then an overflow occurred.
        let set_ov0 = test_bit!((in_1 ^ in_2) & (in_1 ^ result), 15);
        self.set(AccFlags::OV0, set_ov0);
        if set_ov0 {
            self.set(AccFlags::S1, self.contains(AccFlags::S0));
            self.toggle(AccFlags::OV1);
        }
    }

    fn clear_ovf(&mut self) {
        self.remove(AccFlags::OV0 | AccFlags::OV1);
    }

    fn clear_ovf_and_carry(&mut self) {
        self.remove(AccFlags::OV0 | AccFlags::OV1 | AccFlags::C);
    }

    fn carry(self) -> u32 {
        ((self & AccFlags::C).bits() >> 3) as u32
    }
}

bitflags! {
    #[derive(Default)]
    struct StatusFlags: u16 {
        const RQM = bit!(15, u16);   // Request for master
        const USF1 = bit!(14, u16);  // User flag
        const USF0 = bit!(13, u16);
        const DRS = bit!(12, u16);   // DR status
        const DMA = bit!(11, u16);
        const DRC = bit!(10, u16);   // DR 8-bit mode
        const SOC = bit!(9, u16);    // SO 8-bit mode
        const SIC = bit!(8, u16);    // SI 8-bit mode
        const EI = bit!(7, u16);     // Interrupt enable
        const P1 = bit!(1, u16);     // P1 pin
        const P0 = bit!(0, u16);     // P0 pin
    }
}

bitflags! {
    #[derive(Default)]
    struct Instruction: u32 {
        const ALU = bit!(23, u32);  // 0 = is ALU instruction
        const JP = bit!(22, u32);   // 0 = is JP instruction
        // ALU
        const RT = bit!(22, u32);
        const P = bits32![21, 20];
        const ALU_OPCODE = bits32![19, 18, 17, 16];
        const A = bit!(15, u32);
        const DPL = bits32![14, 13];
        const DPH = bits32![12, 11, 10, 9];
        const RP = bit!(8, u32);
        const SRC = bits32![7, 6, 5, 4];
        const DST = bits32![3, 2, 1, 0];
        // LD
        const ID = bits32![21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6];
        // JP
        const BRCH = bits32![21, 20, 19, 18, 17, 16, 15, 14, 13];
        const NA = bits32![12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2];
    }
}

impl Instruction {
    fn is_alu(self) -> bool {
        !self.contains(Instruction::ALU)
    }

    fn is_jp(self) -> bool {
        !self.contains(Instruction::JP)
    }

    fn alu_opcode(self) -> u32 {
        (self & Instruction::ALU_OPCODE).bits() >> 16
    }

    fn use_acc_b(self) -> bool {
        self.contains(Instruction::A)
    }

    fn should_do_alu_op(self, acc_affected: MovedToAcc) -> bool {
        match acc_affected {
            MovedToAcc::No => true,
            MovedToAcc::AccA => self.use_acc_b(),
            MovedToAcc::AccB => !self.use_acc_b(),
        }
    }

    fn p(self) -> u32 {
        (self & Instruction::P).bits() >> 20
    }

    fn should_return(self) -> bool {
        self.contains(Instruction::RT)
    }

    fn dp_adjust(self, dp: u8) -> u8 {
        let dp_lo = match (self & Instruction::DPL).bits() >> 13 {
            0 => dp,
            1 => (dp.wrapping_add(1)) & 0xF,
            2 => (dp.wrapping_sub(1)) & 0xF,
            3 => 0,
            _ => unreachable!()
        };

        let dp_hi_xor = (self & Instruction::DPH).bits() >> 5;
        let dp_hi = (dp_hi_xor as u8) ^ dp;

        (dp_hi & 0xF0) | (dp_lo & 0xF)
    }

    fn should_dec_rp(self) -> bool {
        self.contains(Instruction::RP)
    }

    fn imm_value(self) -> u16 {
        ((self & Instruction::ID).bits() >> 6) as u16
    }

    fn jump_condition(self) -> u32 {
        (self & Instruction::BRCH).bits() >> 13
    }

    // Gives the word address to jump to.
    fn jump_destination(self) -> u16 {
        ((self & Instruction::NA).bits() >> 2) as u16
    }
}

// Did the instruction move data to an accumulator.
enum MovedToAcc {
    No,
    AccA,
    AccB,
}

#[derive(Default)]
pub struct DSP {
    // Registers
    dp:     u8,     // Data RAM pointer
    rp:     u16,    // Data ROM pointer (10-bit)
    pc:     u16,    // Program ROM pointer (11-bit)
    stack:  [u16; 4],   // Stack (11-bit values)
    sp:     u8,     // Stack pointer

    k:      u16,    // Multiplier input
    l:      u16,    // Multiplier input
    m:      u32,    // Multiplier output

    acc_a:  u16,    // Accumulator A
    acc_b:  u16,    // Accumulator B
    flag_a: AccFlags,   // Flags for acc A (6-bit)
    flag_b: AccFlags,   // Flags for acc B (6-bit)

    tr:     u16,    // Temporary storage
    trb:    u16,    // Temporary storage

    sr:     StatusFlags,    // Status I/O register
    dr:     u16,    // Parallel I/O register
    si:     u16,    // Serial I/O data
    so:     u16,    // Serial I/O data

    // Memory
    prog_rom:   Vec<u8>, // 2048 * 24-bit instructions
    data_rom:   Vec<u8>, // 1024 * 16-bit ro-data
    ram:        Vec<u8>, // 256 * 16-bit data
}

impl DSP {
    pub fn new(rom_data: &[u8]) -> Self {
        const PROG_ROM_SIZE: usize = 2048 * 3;
        const DATA_ROM_SIZE: usize = 1024 * 2;

        Self {
            dp: 0,
            rp: 0x3FF,
            pc: 0,
            stack: [0; 4],
            sp: 0,

            k: 0,
            l: 0,
            m: 0,

            acc_a: 0,
            acc_b: 0,
            flag_a: AccFlags::default(),
            flag_b: AccFlags::default(),

            tr: 0,
            trb: 0,
            sr: StatusFlags::default(),
            dr: 0,
            si: 0,
            so: 0,

            prog_rom: Vec::from(&rom_data[0..PROG_ROM_SIZE]),
            data_rom: Vec::from(&rom_data[PROG_ROM_SIZE..(PROG_ROM_SIZE + DATA_ROM_SIZE)]),

            ram: vec![0; 512]
        }
    }

    pub fn step(&mut self) {
        let instr = Instruction::from_bits_truncate(self.fetch_instr());

        if instr.is_alu() {
            self.alu_instr(instr);
        } else {
            if instr.is_jp() {
                self.jp_instr(instr);
            } else {
                self.ld_instr(instr);
            }
        }
    }

    // Set RST pin
    pub fn trigger_reset(&mut self) {
        self.pc = 0;
        self.flag_a = AccFlags::default();
        self.flag_b = AccFlags::default();
        self.sr = StatusFlags::default();
        self.dr = 0;
        // TODO: ack pins = 0
        self.rp = 0x3FF;
    }

    // Set INT pin
    pub fn trigger_int(&mut self) {
        const INT_VECTOR: u16 = 0x100;
        if self.sr.contains(StatusFlags::EI) {
            self.call(INT_VECTOR);
        }
    }

    pub fn read_dr(&mut self) -> u8 {
        if self.sr.contains(StatusFlags::DRC) {
            // 8-bit mode.
            self.sr.remove(StatusFlags::RQM);
            lo!(self.dr)
        } else {
            // 16-bit mode.
            let data = if !self.sr.contains(StatusFlags::DRS) {
                lo!(self.dr)
            } else {
                self.sr.remove(StatusFlags::RQM);
                hi!(self.dr)
            };
            self.sr.toggle(StatusFlags::DRS);
            data
        }
    }

    pub fn write_dr(&mut self, data: u8) {
        if self.sr.contains(StatusFlags::DRC) {
            // 8-bit mode.
            self.sr.remove(StatusFlags::RQM);
            self.dr = set_lo!(self.dr, data);
        } else {
            // 16-bit mode.
            self.dr = if !self.sr.contains(StatusFlags::DRS) {
                set_lo!(self.dr, data)
            } else {
                self.sr.remove(StatusFlags::RQM);
                set_hi!(self.dr, data)
            };
            self.sr.toggle(StatusFlags::DRS);
        }
    }

    pub fn read_sr(&mut self) -> u8 {
        hi!(self.sr.bits())
    }

    pub fn write_sr(&mut self, data: u8) {
        let new_sr = set_hi!(self.sr.bits(), data);
        self.store_sr(new_sr);
    }
}

// Instruction decoding.
impl DSP {
    fn alu_instr(&mut self, instr: Instruction) {
        // Do move.
        let src_data = self.load_idb(instr);
        let acc_affected = self.store_idb(instr, src_data);

        // Do multiply.
        let new_m = (self.k as u32) * (self.l as u32) * 2;

        // TODO: check if operation should happen if RAM is written to AND used in ALU op.
        if instr.should_do_alu_op(acc_affected) {
            // Get ALU op.
            let p = match instr.p() {
                0 => self.load_ram(0),
                1 => src_data,
                2 => hi32!(self.m),
                3 => lo32!(self.m),
                _ => unreachable!()
            };

            // Do ALU instruction.
            match instr.alu_opcode() {
                0x0 => self.nop(),
                0x1 => self.or(instr.use_acc_b(), p),
                0x2 => self.and(instr.use_acc_b(), p),
                0x3 => self.xor(instr.use_acc_b(), p),
                0x4 => self.sub(instr.use_acc_b(), p),
                0x5 => self.add(instr.use_acc_b(), p),
                0x6 => self.sbb(instr.use_acc_b(), p),
                0x7 => self.adc(instr.use_acc_b(), p),
                0x8 => self.sub(instr.use_acc_b(), 1),
                0x9 => self.add(instr.use_acc_b(), 1),
                0xA => self.not(instr.use_acc_b()),
                0xB => self.sar(instr.use_acc_b()),
                0xC => self.rcl(instr.use_acc_b()),
                0xD => self.sll2(instr.use_acc_b()),
                0xE => self.sll4(instr.use_acc_b()),
                0xF => self.xchg(instr.use_acc_b()),
                _ => unreachable!()
            }
        }

        // Modify data pointers.
        self.dp = instr.dp_adjust(self.dp);
        if instr.should_dec_rp() {
            self.rp = self.rp.wrapping_sub(1) & 0x3FF;
        }

        // Write back multiply result.
        self.m = new_m;

        if instr.should_return() {
            self.ret();
        }
    }

    fn jp_instr(&mut self, instr: Instruction) {
        match instr.jump_condition() {
            0x100 => self.jump(instr),
            0x140 => self.call(instr.jump_destination()),

            0x080 if !self.flag_a.contains(AccFlags::C) => self.jump(instr),
            0x082 if self.flag_a.contains(AccFlags::C) => self.jump(instr),
            0x084 if !self.flag_b.contains(AccFlags::C) => self.jump(instr),
            0x086 if self.flag_b.contains(AccFlags::C) => self.jump(instr),

            0x088 if !self.flag_a.contains(AccFlags::Z) => self.jump(instr),
            0x08A if self.flag_a.contains(AccFlags::Z) => self.jump(instr),
            0x08C if !self.flag_b.contains(AccFlags::Z) => self.jump(instr),
            0x08E if self.flag_b.contains(AccFlags::Z) => self.jump(instr),

            0x090 if !self.flag_a.contains(AccFlags::OV0) => self.jump(instr),
            0x092 if self.flag_a.contains(AccFlags::OV0) => self.jump(instr),
            0x094 if !self.flag_b.contains(AccFlags::OV0) => self.jump(instr),
            0x096 if self.flag_b.contains(AccFlags::OV0) => self.jump(instr),

            0x098 if !self.flag_a.contains(AccFlags::OV1) => self.jump(instr),
            0x09A if self.flag_a.contains(AccFlags::OV1) => self.jump(instr),
            0x09C if !self.flag_b.contains(AccFlags::OV1) => self.jump(instr),
            0x09E if self.flag_b.contains(AccFlags::OV1) => self.jump(instr),

            0x0A0 if !self.flag_a.contains(AccFlags::S0) => self.jump(instr),
            0x0A2 if self.flag_a.contains(AccFlags::S0) => self.jump(instr),
            0x0A4 if !self.flag_b.contains(AccFlags::S0) => self.jump(instr),
            0x0A6 if self.flag_b.contains(AccFlags::S0) => self.jump(instr),

            0x0A8 if !self.flag_a.contains(AccFlags::S1) => self.jump(instr),
            0x0AA if self.flag_a.contains(AccFlags::S1) => self.jump(instr),
            0x0AC if !self.flag_b.contains(AccFlags::S1) => self.jump(instr),
            0x0AE if self.flag_b.contains(AccFlags::S1) => self.jump(instr),

            0x0B1 if lo_nybble!(self.dp) == 0x0 => self.jump(instr),
            0x0B2 if lo_nybble!(self.dp) != 0x0 => self.jump(instr),
            0x0B3 if lo_nybble!(self.dp) == 0xF => self.jump(instr),
            0x0B4 if lo_nybble!(self.dp) != 0xF => self.jump(instr),

            0x0B4 => panic!("Trying to use serial ACK"), // TODO: check these
            0x0B6 => panic!("Trying to use serial ACK"),
            0x0B8 => panic!("Trying to use serial ACK"),
            0x0BA => panic!("Trying to use serial ACK"),

            0x0BC if !self.sr.contains(StatusFlags::RQM) => self.jump(instr),
            0x0BE if self.sr.contains(StatusFlags::RQM) => self.jump(instr),

            0x0..=0x1FF => {},  // Undefined 9-bit jump op
            _ => unreachable!()
        }
    }

    fn ld_instr(&mut self, instr: Instruction) {
        let imm = instr.imm_value();
        self.store_idb(instr, imm);
        
        self.m = (self.k as u32) * (self.l as u32) * 2;
    }
}

// ALU instructions.
impl DSP {
    fn nop(&mut self) {
        
    }

    fn or(&mut self, use_acc_b: bool, p: u16) {
        if use_acc_b {
            self.acc_b = self.acc_b | p;
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf_and_carry();
        } else {
            self.acc_a = self.acc_a | p;
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf_and_carry();
        }
    }

    fn and(&mut self, use_acc_b: bool, p: u16) {
        if use_acc_b {
            self.acc_b = self.acc_b & p;
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf_and_carry();
        } else {
            self.acc_a = self.acc_a & p;
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf_and_carry();
        }
    }

    fn xor(&mut self, use_acc_b: bool, p: u16) {
        if use_acc_b {
            self.acc_b = self.acc_b ^ p;
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf_and_carry();
        } else {
            self.acc_a = self.acc_a ^ p;
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf_and_carry();
        }
    }

    fn not(&mut self, use_acc_b: bool) {
        if use_acc_b {
            self.acc_b = !self.acc_b;
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf_and_carry();
        } else {
            self.acc_a = !self.acc_a;
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf_and_carry();
        }
    }

    fn add(&mut self, use_acc_b: bool, p: u16) {
        if use_acc_b {
            let result = (self.acc_b as u32) + (p as u32);
            let result16 = lo32!(result);
            self.flag_b.set_sz(result16);
            self.flag_b.set(AccFlags::C, result > (std::u16::MAX as u32));
            self.flag_b.set_add_ovf(self.acc_b, p, result16);
            self.acc_b = result16;
        } else {
            let result = (self.acc_a as u32) + (p as u32);
            let result16 = lo32!(result);
            self.flag_a.set_sz(result16);
            self.flag_a.set(AccFlags::C, result > (std::u16::MAX as u32));
            self.flag_a.set_add_ovf(self.acc_a, p, result16);
            self.acc_a = result16;
        }
    }

    fn sub(&mut self, use_acc_b: bool, p: u16) {
        if use_acc_b {
            let result = ((self.acc_b as i16) as i32) - ((p as i16) as i32);
            let result16 = lo32!(result as u32);
            self.flag_b.set_sz(result16);
            self.flag_b.set(AccFlags::C, result < 0);
            self.flag_b.set_sub_ovf(self.acc_b, p, result16);
            self.acc_b = result16;
        } else {
            let result = ((self.acc_a as i16) as i32) - ((p as i16) as i32);
            let result16 = lo32!(result as u32);
            self.flag_a.set_sz(result16);
            self.flag_a.set(AccFlags::C, result < 0);
            self.flag_a.set_sub_ovf(self.acc_a, p, result16);
            self.acc_a = result16;
        }
    }

    fn adc(&mut self, use_acc_b: bool, p: u16) {
        if use_acc_b {
            let result = (self.acc_b as u32) + (p as u32) + self.flag_a.carry();
            let result16 = lo32!(result);
            self.flag_b.set_sz(result16);
            self.flag_b.set(AccFlags::C, result > (std::u16::MAX as u32));
            self.flag_b.set_add_ovf(self.acc_b, p, result16);
            self.acc_b = result16;
        } else {
            let result = (self.acc_a as u32) + (p as u32) + self.flag_b.carry();
            let result16 = lo32!(result);
            self.flag_a.set_sz(result16);
            self.flag_a.set(AccFlags::C, result > (std::u16::MAX as u32));
            self.flag_a.set_add_ovf(self.acc_a, p, result16);
            self.acc_a = result16;
        }
    }

    fn sbb(&mut self, use_acc_b: bool, p: u16) {
        if use_acc_b {
            let result = ((self.acc_b as i16) as i32) - ((p as i16) as i32) - (self.flag_a.carry() as i32);
            let result16 = lo32!(result as u32);
            self.flag_b.set_sz(result16);
            self.flag_b.set(AccFlags::C, result < 0);
            self.flag_b.set_sub_ovf(self.acc_b, p, result16);
            self.acc_b = result16;
        } else {
            let result = ((self.acc_a as i16) as i32) - ((p as i16) as i32) - (self.flag_b.carry() as i32);
            let result16 = lo32!(result as u32);
            self.flag_a.set_sz(result16);
            self.flag_a.set(AccFlags::C, result < 0);
            self.flag_a.set_sub_ovf(self.acc_a, p, result16);
            self.acc_a = result16;
        }
    }

    fn sar(&mut self, use_acc_b: bool) {
        if use_acc_b {
            let signed_acc = self.acc_b as i16;
            self.flag_b.set(AccFlags::C, test_bit!(self.acc_b, 0));
            self.acc_b = (signed_acc >> 1) as u16;
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf();
        } else {
            let signed_acc = self.acc_a as i16;
            self.flag_a.set(AccFlags::C, test_bit!(self.acc_a, 0));
            self.acc_a = (signed_acc >> 1) as u16;
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf();
        }
    }

    fn rcl(&mut self, use_acc_b: bool) {
        if use_acc_b {
            self.flag_b.set(AccFlags::C, test_bit!(self.acc_b, 15));
            self.acc_b = (self.acc_b << 1) | (self.flag_a.carry() as u16);
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf();
        } else {
            self.flag_a.set(AccFlags::C, test_bit!(self.acc_a, 15));
            self.acc_a = (self.acc_a << 1) | (self.flag_b.carry() as u16);
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf();
        }
    }

    fn sll2(&mut self, use_acc_b: bool) {
        if use_acc_b {
            self.acc_b = (self.acc_b << 2) | 0x3;
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf_and_carry();
        } else {
            self.acc_a = (self.acc_a << 2) | 0x3;
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf_and_carry();
        }
    }

    fn sll4(&mut self, use_acc_b: bool) {
        if use_acc_b {
            self.acc_b = (self.acc_b << 4) | 0xF;
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf_and_carry();
        } else {
            self.acc_a = (self.acc_a << 4) | 0xF;
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf_and_carry();
        }
    }

    fn xchg(&mut self, use_acc_b: bool) {
        if use_acc_b {
            self.acc_b = make16!(lo!(self.acc_b), hi!(self.acc_b));
            self.flag_b.set_sz(self.acc_b);
            self.flag_b.clear_ovf_and_carry();
        } else {
            self.acc_a = make16!(lo!(self.acc_a), hi!(self.acc_a));
            self.flag_a.set_sz(self.acc_a);
            self.flag_a.clear_ovf_and_carry();
        }
    }
}

// Jump
impl DSP {
    fn jump(&mut self, instr: Instruction) {
        self.pc = instr.jump_destination() * 3;
    }

    fn call(&mut self, dest: u16) {
        self.stack[self.sp as usize] = self.pc;
        self.sp = (self.sp + 1) & 0x3;
        self.pc = dest * 3;
    }

    fn ret(&mut self) {
        self.sp = (self.sp - 1) & 0x3;
        self.pc = self.stack[self.sp as usize];
    }
}

// Helpers
impl DSP {
    // Get a single byte from program memory.
    fn fetch_prog_byte(&mut self) -> u8 {
        const PC_MASK: u16 = bit!(12, u16) - 1;
        let data = self.prog_rom[self.pc as usize];
        self.pc = (self.pc + 1) & PC_MASK;
        data
    }

    // Get an instructon from program memory.
    fn fetch_instr(&mut self) -> u32 {
        let lo = self.fetch_prog_byte();
        let mid = self.fetch_prog_byte();
        let hi = self.fetch_prog_byte();

        make24!(hi, mid, lo)
    }

    // Load data from RAM
    fn load_ram(&self, or: u8) -> u16 {
        let byte_pointer = ((self.dp | or) as usize) << 1;
        let lo = self.ram[byte_pointer];
        let hi = self.ram[byte_pointer + 1];

        make16!(hi, lo)
    }

    // Store data to RAM
    fn store_ram(&mut self, data: u16) {
        let byte_pointer = (self.dp as usize) << 1;
        self.ram[byte_pointer] = lo!(data);
        self.ram[byte_pointer + 1] = hi!(data);
    }

    // Load data from ROM
    fn load_rom(&self) -> u16 {
        let byte_pointer = (self.rp as usize) << 1;
        let lo = self.data_rom[byte_pointer];
        let hi = self.data_rom[byte_pointer + 1];

        make16!(hi, lo)
    }

    // Load data from internal bus.
    fn load_idb(&mut self, instr: Instruction) -> u16 {
        match (instr & Instruction::SRC).bits() >> 4 {
            0x0 => self.trb,
            0x1 => self.acc_a,
            0x2 => self.acc_b,
            0x3 => self.tr,
            0x4 => self.dp as u16,
            0x5 => self.rp,
            0x6 => self.load_rom(),
            0x7 => 0, // SGN ??
            0x8 => self.load_dr(false),
            0x9 => self.load_dr(true),
            0xA => self.sr.bits(),
            0xB => self.load_s_msb(),
            0xC => self.load_s_lsb(),
            0xD => self.k,
            0xE => self.l,
            0xF => self.load_ram(0),
            _ => unreachable!()
        }
    }

    // Store data to internal bus.
    fn store_idb(&mut self, instr: Instruction, data: u16) -> MovedToAcc {
        match (instr & Instruction::DST).bits() {
            0x0 => {},
            0x1 => {
                self.acc_a = data;
                return MovedToAcc::AccA;
            },
            0x2 => {
                self.acc_b = data;
                return MovedToAcc::AccB;
            },
            0x3 => self.tr = data,
            0x4 => self.dp = lo!(data),
            0x5 => self.rp = data & 0x3FF,
            0x6 => self.store_dr(data),
            0x7 => self.store_sr(data),
            0x8 => self.store_s_lsb(data),
            0x9 => self.store_s_msb(data),
            0xA => self.k = data,
            0xB => {
                self.k = data;
                self.l = self.load_rom();
            },
            0xC => {
                self.l = data;
                self.k = self.load_ram(0x40);
            },
            0xD => self.l = data,
            0xE => self.trb = data,
            0xF => self.store_ram(data),
            _ => unreachable!()
        }
        MovedToAcc::No
    }

    // Read from parallel I/O port.
    fn load_dr(&mut self, nf: bool) -> u16 {
        if !nf {
            self.sr.insert(StatusFlags::RQM);
        }
        if self.sr.contains(StatusFlags::DRC) {
            make16!(0, lo!(self.dr))
        } else {
            self.dr
        }
    }

    // Write to parallel I/O port.
    fn store_dr(&mut self, data: u16) {
        self.sr.insert(StatusFlags::RQM);
        if self.sr.contains(StatusFlags::DRC) {
            self.dr = lo!(data);
        } else {
            self.dr = data;
        }
    }

    // Write to status register.
    fn store_sr(&mut self, data: u16) {
        let rqm = self.sr.contains(StatusFlags::RQM);
        let drs = self.sr.contains(StatusFlags::DRS);
        self.sr = StatusFlags::from_bits_truncate(data);
        self.sr.set(StatusFlags::RQM, rqm);
        self.sr.set(StatusFlags::DRS, drs);
    }

    // Read from serial I/O port MSB first.
    fn load_s_msb(&mut self) -> u16 {
        0
    }

    // Read from serial I/O port LSB first.
    fn load_s_lsb(&mut self) -> u16 {
        0
    }

    // Write to serial I/O port MSB first.
    fn store_s_msb(&mut self, data: u16) {
        
    }

    // Write to serial I/O port LSB first.
    fn store_s_lsb(&mut self, data: u16) {
        
    }
}