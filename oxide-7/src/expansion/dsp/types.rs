use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct AccFlags: u8 {
        const S0 = bit!(5);     // Sign
        const Z = bit!(4);      // Zero
        const C = bit!(3);      // Carry
        const OV0 = bit!(2);    // Overflow
        const S1 = bit!(1);     // Direction of overflow
        const OV1 = bit!(0);    // Number of overflows
    }
}

impl AccFlags {
    pub fn set_sz(&mut self, result: u16) {
        // TODO: Should set s1 if not ovf instruction
        self.set(AccFlags::S0, test_bit!(result, 15));
        self.set(AccFlags::Z, result == 0);
    }

    pub fn set_add_ovf(&mut self, in_1: u16, in_2: u16, result: u16) {
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

    pub fn set_sub_ovf(&mut self, in_1: u16, in_2: u16, result: u16) {
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

    pub fn clear_ovf(&mut self) {
        self.remove(AccFlags::OV0 | AccFlags::OV1);
    }

    pub fn clear_ovf_and_carry(&mut self) {
        self.remove(AccFlags::OV0 | AccFlags::OV1 | AccFlags::C);
    }

    pub fn carry(self) -> u32 {
        ((self & AccFlags::C).bits() >> 3) as u32
    }
}

bitflags! {
    #[derive(Default)]
    pub struct StatusFlags: u16 {
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
    pub struct Instruction: u32 {
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
    pub fn is_alu(self) -> bool {
        !self.contains(Instruction::ALU)
    }

    pub fn is_jp(self) -> bool {
        !self.contains(Instruction::JP)
    }

    pub fn alu_opcode(self) -> u32 {
        (self & Instruction::ALU_OPCODE).bits() >> 16
    }

    pub fn use_acc_b(self) -> bool {
        self.contains(Instruction::A)
    }

    pub fn should_do_alu_op(self, acc_affected: MovedToAcc) -> bool {
        match acc_affected {
            MovedToAcc::No => true,
            MovedToAcc::AccA => self.use_acc_b(),
            MovedToAcc::AccB => !self.use_acc_b(),
        }
    }

    pub fn p(self) -> u32 {
        (self & Instruction::P).bits() >> 20
    }

    pub fn should_return(self) -> bool {
        self.contains(Instruction::RT)
    }

    pub fn dp_adjust(self, dp: u8) -> u8 {
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

    pub fn should_dec_rp(self) -> bool {
        self.contains(Instruction::RP)
    }

    pub fn imm_value(self) -> u16 {
        ((self & Instruction::ID).bits() >> 6) as u16
    }

    pub fn jump_condition(self) -> u32 {
        (self & Instruction::BRCH).bits() >> 13
    }

    // Gives the word address to jump to.
    pub fn jump_destination(self) -> u16 {
        ((self & Instruction::NA).bits() >> 2) as u16
    }
}

// Did the instruction move data to an accumulator.
pub enum MovedToAcc {
    No,
    AccA,
    AccB,
}
