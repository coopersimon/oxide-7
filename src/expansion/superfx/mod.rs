// MARIO Chip, Super FX and Super FX 2 (GSU)
mod cache;
mod mem;

use bitflags::bitflags;

use crate::{
    common::Interrupt,
    mem::rom::{ROM, SRAM}
};

use cache::*;
use mem::FXMem;
use super::Expansion;

bitflags! {
    #[derive(Default)]
    struct FXFlags: u16 {
        const IRQ = bit!(15, u16);  // Interrupt
        const B = bit!(12, u16);    // Prefix
        const IH = bit!(11, u16);   // Immediate upper
        const IL = bit!(10, u16);   // Immediate lower
        const ALT2 = bit!(9, u16);  // Prefix
        const ALT1 = bit!(8, u16);  // Prefix
        const R = bit!(6, u16);     // ROM Read ?
        const GO = bit!(5, u16);    // Running
        const OV = bit!(4, u16);    // Overflow
        const S = bit!(3, u16);     // Sign
        const CY = bit!(2, u16);    // Carry
        const Z = bit!(1, u16);     // Zero
    }
}

bitflags! {
    #[derive(Default)]
    struct Config: u8 {
        const IRQ = bit!(7);
        const MS0 = bit!(5);
    }
}

bitflags! {
    #[derive(Default)]
    struct ScreenMode: u8 {
        const HT1 = bit!(5);
        const RON = bit!(4);
        const RAN = bit!(3);
        const HT0 = bit!(2);
        const MD = bits![1, 0];
    }
}

bitflags! {
    #[derive(Default)]
    struct PlotOption: u8 {
        const OBJ_MODE = bit!(4);
        const FREEZE_HI = bit!(3);
        const HI_NYBBLE = bit!(2);
        const DITHER = bit!(1);
        const TRANSPARENT = bit!(0);
    }
}

const ROM_PTR_REG: usize = 14;
const PC_REG: usize = 15;

pub struct SuperFX {
    regs:       [u16; 16],
    regs_latch: u8,
    pc:         u16, // PC of next instruction.

    flags:      FXFlags,
    pb:         u8,
    romb:       u8,
    ramb:       u8,
    //bram:       u8,
    cfg:        Config,
    last_ram_addr:  u16,

    src:        usize,
    dst:        usize,

    // TODO: move?
    screen_base:    u8,
    screen_mode:    ScreenMode,
    colr:           u8,
    por:            PlotOption,

    version:        u8,
    clock_select:   bool,

    cache:          Cache,
    mem:            FXMem,

    cycle_count:    isize,
}

impl SuperFX {
    pub fn new(rom: ROM, sram: Box<dyn SRAM>) -> Self {
        SuperFX {
            regs:       [0; 16],
            regs_latch: 0,
            pc:         0,

            flags:      FXFlags::default(),
            pb:         0,
            romb:       0,
            ramb:       0,
            //bram:       0,
            cfg:        Config::default(),
            last_ram_addr:  0,

            src:        0,
            dst:        0,

            screen_base:    0,
            screen_mode:    ScreenMode::default(),
            colr:           0,
            por:            PlotOption::default(),

            version:        0,
            clock_select:   false,

            cache:          Cache::new(),
            mem:            FXMem::new(rom, sram),

            cycle_count:    0,
        }
    }

    fn step(&mut self) {
        self.execute_instruction();
    }
}

impl Expansion for SuperFX {
    fn read(&mut self, bank: u8, addr: u16) -> u8 {
        match bank % 0x80 {
            0x00..=0x3F if addr <= 0x3500 => self.read_reg(addr),
            _ => self.mem.snes_read(bank, addr)
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) {
        match bank % 0x80 {
            0x00..=0x3F if addr <= 0x3500 => self.write_reg(addr, data),
            _ => self.mem.snes_write(bank, addr, data)
        }
    }

    fn clock(&mut self, cycles: usize) -> Interrupt {
        // Convert master cycles to FX cycles.
        let fx_cycles = if self.clock_select {cycles} else {cycles / 2};
        self.cycle_count -= fx_cycles as isize;

        while self.cycle_count <= 0 {
            self.step();
        }

        Interrupt::default()
    }

    fn flush(&mut self) {
        self.mem.flush();
    }
}

// Registers
impl SuperFX {
    fn read_reg(&mut self, addr: u16) -> u8 {
        match addr {
            0x3000..=0x301F => if test_bit!(addr, 0) {
                let addr = (addr % 0x20) >> 1;
                hi!(self.regs[addr as usize])
            } else {
                let addr = (addr % 0x20) >> 1;
                lo!(self.regs[addr as usize])
            },
            0x3030 => lo!(self.flags.bits()),
            0x3031 => hi!(self.flags.bits()),
            0x3034 => self.pb,
            0x3036 => self.romb,
            0x303B => self.version,
            0x303C => self.ramb,
            0x303E => lo!(self.cache.get_cbr()),
            0x303F => hi!(self.cache.get_cbr()),

            0x3100..=0x32FF => self.cache.read(addr - 0x3100),
            _ => 0,
        }
    }

    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0x3000..=0x301E => if test_bit!(addr, 0) {
                let addr = (addr % 0x20) >> 1;
                self.regs[addr as usize] = make16!(data, self.regs_latch);
            } else {
                self.regs_latch = data;
            },
            0x301F => {
                self.regs[PC_REG] = make16!(data, self.regs_latch);
                // start
            },
            0x3030 => self.set_status_flags(data),
            0x3034 => self.pb = data,
            0x3037 => self.cfg = Config::from_bits_truncate(data),
            0x3039 => self.clock_select = test_bit!(data, 0, u8),
            
            0x3038 => self.screen_base = data,
            0x303A => self.screen_mode = ScreenMode::from_bits_truncate(data),

            0x3100..=0x32FF => self.cache.write(addr - 0x3100, data),
            _ => {},
        }
        
        
    }

    fn set_status_flags(&mut self, data: u8) {
        self.flags.set(FXFlags::Z, test_bit!(data, 1, u8));
        self.flags.set(FXFlags::CY, test_bit!(data, 2, u8));
        self.flags.set(FXFlags::S, test_bit!(data, 3, u8));
        self.flags.set(FXFlags::OV, test_bit!(data, 4, u8));
        self.flags.set(FXFlags::GO, test_bit!(data, 5, u8));    // TODO: start/stop
        if !test_bit!(data, 5, u8) {
            self.cache.set_cbr(0);
            // TODO: fill start?
        }
    }
}

// Instructions
impl SuperFX {
    fn execute_instruction(&mut self) {
        let instr = self.fetch();

        match hi_nybble!(instr) {
            0x0 => match lo_nybble!(instr) {
                0x0 => self.stop(),
                0x1 => self.nop(),
                0x2 => self.cache(),
                0x3 => self.lsr(),
                0x4 => self.rol(),
                0x5..=0xF => self.branch(instr),
                _ => unreachable!(),
            },
            0x1 => if self.flags.contains(FXFlags::B) {
                self.mov(instr)
            } else {
                self.to(instr)
            },
            0x2 => self.with(instr),
            0x3 => match lo_nybble!(instr) {
                n @ 0x0..=0xB => self.st(n as usize),
                0xC => self.loop_(),
                0xD => self.flags.insert(FXFlags::ALT1),
                0xE => self.flags.insert(FXFlags::ALT2),
                0xF => self.flags.insert(FXFlags::ALT1 | FXFlags::ALT2),
                _ => unreachable!(),
            },
            0x4 => match lo_nybble!(instr) {
                n @ 0x0..=0xB => self.ld(n as usize),
                0xC => self.pix(),
                0xD => self.swap(),
                0xE => self.creg(),
                0xF => self.not(),
                _ => unreachable!(),
            },
            0x5 => self.add(instr),
            0x6 => self.sub(instr),
            0x7 => match lo_nybble!(instr) {
                0 => self.merge(),
                n => self.logic_7(n),
            },
            0x8 => self.mult_byte(lo_nybble!(instr)),
            0x9 => match lo_nybble!(instr) {
                0x0 => self.sbk(),
                n @ 0x1..=0x4 => self.link(n),
                0x5 => self.sex(),
                0x6 => self.asr(),
                0x7 => self.ror(),
                n @ 0x8..=0xD => self.jmp(n),
                0xE => self.lob(),
                0xF => self.mult_word(),
                _ => unreachable!(),
            },
            0xA => match self.alt() {
                0 => self.ibt(instr),
                1 => self.lms(instr),
                2 => self.sms(instr),
                3 => self.reset_prefix(),
                _ => unreachable!(),
            },
            0xB => if self.flags.contains(FXFlags::B) {
                self.moves(instr)
            } else {
                self.from(instr)
            },
            0xC => match lo_nybble!(instr) {
                0 => self.hib(),
                n => self.logic_c(n),
            },
            0xD => match lo_nybble!(instr) {
                0xF => self.reg_mov(),
                n => self.inc(n),
            },
            0xE => match lo_nybble!(instr) {
                0xF => self.getb(),
                n => self.dec(n),
            },
            0xF => match self.alt() {
                0 => self.iwt(instr),
                1 => self.lm(instr),
                2 => self.sm(instr),
                3 => self.reset_prefix(),
                _ => unreachable!(),
            },

            _ => unreachable!(),
        }
    }
}

// Prefixes
impl SuperFX {
    fn to(&mut self, instr: u8) {
        let reg = lo_nybble!(instr);
        self.dst = reg as usize;
    }

    fn from(&mut self, instr: u8) {
        let reg = lo_nybble!(instr);
        self.src = reg as usize;
    }

    fn with(&mut self, instr: u8) {
        let reg = lo_nybble!(instr);
        self.src = reg as usize;
        self.dst = reg as usize;
        self.flags.insert(FXFlags::B);
    }

    fn reset_prefix(&mut self) {
        self.src = 0;
        self.dst = 0;
        self.flags.remove(FXFlags::B | FXFlags::ALT1 | FXFlags::ALT2);
    }
}

// Special
impl SuperFX {
    fn stop(&mut self) {
        let _ = self.fetch();
        self.flags.remove(FXFlags::GO);
        self.flags.insert(FXFlags::IRQ);
    }

    fn nop(&mut self) {

    }

    fn cache(&mut self) {
        self.cache.set_cbr(self.regs[PC_REG] & 0xFFF0);
        self.fill_cache_line_start();
    }
}

// Jump / branch
impl SuperFX {
    fn branch(&mut self, instr: u8) {
        let offset = self.fetch() as i8;
        match lo_nybble!(instr) {
            0x5 => self.do_branch(offset),
            0x6 => {
                let s = self.flags.contains(FXFlags::S);
                let v = self.flags.contains(FXFlags::OV);
                if s == v {
                    self.do_branch(offset);
                }
            },
            0x7 => {
                let s = self.flags.contains(FXFlags::S);
                let v = self.flags.contains(FXFlags::OV);
                if s != v {
                    self.do_branch(offset);
                }
            },
            0x8 if !self.flags.contains(FXFlags::Z) => self.do_branch(offset),
            0x9 if self.flags.contains(FXFlags::Z) => self.do_branch(offset),
            0xA if !self.flags.contains(FXFlags::S) => self.do_branch(offset),
            0xB if self.flags.contains(FXFlags::S) => self.do_branch(offset),
            0xC if !self.flags.contains(FXFlags::CY) => self.do_branch(offset),
            0xD if self.flags.contains(FXFlags::CY) => self.do_branch(offset),
            0xE if !self.flags.contains(FXFlags::OV) => self.do_branch(offset),
            0xF if self.flags.contains(FXFlags::OV) => self.do_branch(offset),
            _ => {}
        }
    }

    fn do_branch(&mut self, offset: i8) {
        let offset16 = (offset as i16) as u16;
        self.set_pc_reg(self.regs[PC_REG].wrapping_add(offset16));
    }

    fn jmp(&mut self, n: u8) {
        if self.flags.contains(FXFlags::ALT1) {
            self.regs[PC_REG] = self.regs[self.src];
            self.pb = lo!(self.regs[n as usize]);
            self.cache();
        } else {
            self.set_pc_reg(self.regs[n as usize]);
        }
        self.reset_prefix();
    }

    fn loop_(&mut self) {
        let dec = self.regs[12].wrapping_sub(1);
        self.flags.set(FXFlags::S, test_bit!(dec, 15));
        self.flags.set(FXFlags::Z, dec == 0);
        if !self.flags.contains(FXFlags::Z) {
            self.set_pc_reg(self.regs[13]);
        }
        self.reset_prefix();
    }

    fn link(&mut self, n: u8) {
        self.regs[11] = self.regs[PC_REG].wrapping_add(n as u16);
        self.reset_prefix();
    }
}

// Moves
impl SuperFX {
    fn mov(&mut self, instr: u8) {
        self.dst = lo_nybble!(instr) as usize;
        self.set_dst_reg(self.regs[self.src]);
        self.reset_prefix();
    }

    fn moves(&mut self, instr: u8) {
        self.dst = lo_nybble!(instr) as usize;
        let data = self.regs[self.src];
        self.flags.set(FXFlags::OV, test_bit!(data, 7));
        self.flags.set(FXFlags::S, test_bit!(data, 15));
        self.flags.set(FXFlags::Z, data == 0);
        self.set_dst_reg(data);
        self.reset_prefix();
    }

    fn ibt(&mut self, instr: u8) {
        let dst = lo_nybble!(instr) as usize;
        let data = (self.fetch() as i8) as i16;
        if dst == PC_REG {
            self.set_pc_reg(data as u16);
        } else {
            self.regs[dst] = data as u16;
        }
        self.reset_prefix();
    }

    fn iwt(&mut self, instr: u8) {
        let dst = lo_nybble!(instr) as usize;
        let data_lo = self.fetch();
        let data_hi = self.fetch();
        let data = make16!(data_hi, data_lo);
        if dst == PC_REG {
            self.set_pc_reg(data);
        } else {
            self.regs[dst] = data;
        }
        self.reset_prefix();
    }

    fn getb(&mut self) {
        let data = self.read_mem(self.romb, self.regs[ROM_PTR_REG]);

        self.set_dst_reg(match self.alt() {
            0 => data as u16,
            1 => set_hi!(self.regs[self.dst], data),
            2 => set_lo!(self.regs[self.dst], data),
            3 => ((data as i8) as i16) as u16,
            _ => unreachable!(),
        });

        self.reset_prefix();
    }

    fn ld(&mut self, n: usize) {
        self.last_ram_addr = self.regs[n];

        let data = if self.flags.contains(FXFlags::ALT1) {
            let lo = self.read_mem(self.ramb, self.last_ram_addr);
            make16!(0, lo)
        } else {
            let lo = self.read_mem(self.ramb, self.last_ram_addr);
            let hi = self.read_mem(self.ramb, self.last_ram_addr.wrapping_add(1));
            make16!(hi, lo)
        };

        self.set_dst_reg(data);

        self.reset_prefix();
    }

    fn lm(&mut self, instr: u8) {
        let lo = self.fetch();
        let hi = self.fetch();
        self.last_ram_addr = make16!(hi, lo);

        let dst = lo_nybble!(instr) as usize;
        let lo = self.read_mem(self.ramb, self.last_ram_addr);
        let hi = self.read_mem(self.ramb, self.last_ram_addr.wrapping_add(1));
        self.regs[dst] = make16!(hi, lo);

        self.reset_prefix();
    }

    fn lms(&mut self, instr: u8) {
        self.last_ram_addr = (self.fetch() as u16) << 1;
        
        let dst = lo_nybble!(instr) as usize;
        let lo = self.read_mem(self.ramb, self.last_ram_addr);
        let hi = self.read_mem(self.ramb, self.last_ram_addr.wrapping_add(1));
        self.regs[dst] = make16!(hi, lo);

        self.reset_prefix();
    }

    fn st(&mut self, n: usize) {
        self.last_ram_addr = self.regs[n];

        if self.flags.contains(FXFlags::ALT1) {
            self.write_mem(self.ramb, self.last_ram_addr, lo!(self.regs[self.src]));
        } else {
            self.write_mem(self.ramb, self.last_ram_addr, lo!(self.regs[self.src]));
            self.write_mem(self.ramb, self.last_ram_addr.wrapping_add(1), hi!(self.regs[self.src]));
        }

        self.reset_prefix();
    }

    fn sm(&mut self, instr: u8) {
        let lo = self.fetch();
        let hi = self.fetch();
        self.last_ram_addr = make16!(hi, lo);

        let src = lo_nybble!(instr) as usize;
        self.write_mem(self.ramb, self.last_ram_addr, lo!(self.regs[src]));
        self.write_mem(self.ramb, self.last_ram_addr.wrapping_add(1), hi!(self.regs[src]));

        self.reset_prefix();
    }

    fn sms(&mut self, instr: u8) {
        self.last_ram_addr = (self.fetch() as u16) << 1;

        let src = lo_nybble!(instr) as usize;
        self.write_mem(self.ramb, self.last_ram_addr, lo!(self.regs[src]));
        self.write_mem(self.ramb, self.last_ram_addr.wrapping_add(1), hi!(self.regs[src]));

        self.reset_prefix();
    }

    fn sbk(&mut self) {
        self.write_mem(self.ramb, self.last_ram_addr, lo!(self.regs[self.src]));
        self.write_mem(self.ramb, self.last_ram_addr.wrapping_add(1), hi!(self.regs[self.src]));
    }

    fn reg_mov(&mut self) {
        match self.alt() {
            0 | 1 => self.colr = self.read_mem(self.romb, self.regs[ROM_PTR_REG]),
            2 => self.ramb = lo!(self.regs[self.src]) & 0x1,
            3 => self.romb = lo!(self.regs[self.src]),
            _ => unreachable!(),
        }

        self.reset_prefix();
    }
}

// Byte ops
impl SuperFX {
    fn swap(&mut self) {
        let s = self.regs[self.src];
        let result = make16!(lo!(s), hi!(s));
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);

        self.reset_prefix();
    }

    fn sex(&mut self) {
        let s = self.regs[self.src];
        let result = ((lo!(s) as i8) as i16) as u16;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);

        self.reset_prefix();
    }

    fn lob(&mut self) {
        let s = self.regs[self.src];
        let result = make16!(0, lo!(s));
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);

        self.reset_prefix();
    }

    fn hib(&mut self) {
        let s = self.regs[self.src];
        let result = make16!(0, hi!(s));
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);

        self.reset_prefix();
    }

    fn merge(&mut self) {
        let result = make16!(hi!(self.regs[7]), hi!(self.regs[8]));
        self.flags.set(FXFlags::Z, (result & 0xF0F0) != 0);
        self.flags.set(FXFlags::CY, (result & 0xE0E0) != 0);
        self.flags.set(FXFlags::S, (result & 0x8080) != 0);
        self.flags.set(FXFlags::OV, (result & 0xC0C0) != 0);
        self.set_dst_reg(result);

        self.reset_prefix();
    }
}

// Bitmap
impl SuperFX {
    fn creg(&mut self) {
        if self.flags.contains(FXFlags::ALT1) {
            self.por = PlotOption::from_bits_truncate(lo!(self.regs[self.src]));
        } else {
            self.colr = lo!(self.regs[self.src]);
        }

        self.reset_prefix();
    }

    fn pix(&mut self) {
        if self.flags.contains(FXFlags::ALT1) {
            // RPIX
        } else {
            // PLOT
        }

        self.reset_prefix();
    }
}

// Arithmetic
impl SuperFX {
    fn add(&mut self, instr: u8) {
        let n = lo_nybble!(instr);
        let result = match self.alt() {
            0 => self.bin_arith(self.regs[n as usize], false),
            1 => self.bin_arith(self.regs[n as usize], true),
            2 => self.bin_arith(n as u16, false),
            3 => self.bin_arith(n as u16, true),
            _ => unreachable!(),
        };
        self.set_dst_reg(result);
        self.reset_prefix();
    }

    fn sub(&mut self, instr: u8) {
        let n = lo_nybble!(instr);
        match self.alt() {
            0 => {
                let data = self.bin_arith(!self.regs[n as usize], false);
                self.set_dst_reg(data);
            },
            1 => {
                let data = self.bin_arith(!self.regs[n as usize], true);
                self.set_dst_reg(data);
            },
            2 => {
                let data = self.bin_arith(!(n as u16), false);
                self.set_dst_reg(data);
            },
            3 => {let _ = self.bin_arith(!(n as u16), false);}, // CMP
            _ => unreachable!(),
        }
        self.reset_prefix();
    }

    fn inc(&mut self, n: u8) {
        let result = self.regs[n as usize].wrapping_add(1);
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.regs[n as usize] = result;
    }

    fn dec(&mut self, n: u8) {
        let result = self.regs[n as usize].wrapping_sub(1);
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.regs[n as usize] = result;
    }

    fn bin_arith(&mut self, op_n: u16, with_carry: bool) -> u16 {
        let op0 = self.regs[self.src] as u32;
        let op1 = op_n as u32;
        let carry = if with_carry && self.flags.contains(FXFlags::CY) {1} else {0};
        let result = op0.wrapping_add(op1).wrapping_add(carry);
        self.flags.set(FXFlags::Z, lo32!(result) == 0);
        self.flags.set(FXFlags::CY, test_bit!(result, 16, u32));
        self.flags.set(FXFlags::S, test_bit!(result, 15, u32));
        self.flags.set(FXFlags::OV, test_bit!(!(op0 ^ op1) & (op0 ^ result), 15, u32));
        lo32!(result)
    }

}

// Logic
impl SuperFX {

    // AND and BIC
    fn logic_7(&mut self, n: u8) {
        match self.alt() {
            0 => self.and(self.regs[n as usize]),
            1 => self.bic(self.regs[n as usize]),
            2 => self.and(n as u16),
            3 => self.bic(n as u16),
            _ => unreachable!(),
        }
        self.reset_prefix();
    }

    // OR and XOR
    fn logic_c(&mut self, n: u8) {
        match self.alt() {
            0 => self.or(self.regs[n as usize]),
            1 => self.xor(self.regs[n as usize]),
            2 => self.or(n as u16),
            3 => self.xor(n as u16),
            _ => unreachable!(),
        }
        self.reset_prefix();
    }

    fn and(&mut self, op_n: u16) {
        let result = self.regs[self.src] & op_n;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);
    }

    fn bic(&mut self, op_n: u16) {
        let result = self.regs[self.src] & !op_n;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);
    }

    fn or(&mut self, op_n: u16) {
        let result = self.regs[self.src] | op_n;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);
    }

    fn xor(&mut self, op_n: u16) {
        let result = self.regs[self.src] ^ op_n;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);
    }

    fn not(&mut self) {
        self.set_dst_reg(!self.regs[self.src]);

        self.reset_prefix();
    }

    fn lsr(&mut self) {
        let s = self.regs[self.src];
        let result = s >> 1;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::CY, test_bit!(s, 0));
        self.flags.set(FXFlags::S, false);
        self.set_dst_reg(result);

        self.reset_prefix();
    }

    fn asr(&mut self) {
        let s = self.regs[self.src];
        let result = if self.flags.contains(FXFlags::ALT1) && s == std::u16::MAX {
            0
        } else {
            ((s as i16) >> 1) as u16
        };

        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::CY, test_bit!(s as u16, 0));
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);

        self.reset_prefix();
    }

    fn rol(&mut self) {
        let s = self.regs[self.src];
        let result = (s << 1) | if self.flags.contains(FXFlags::CY) {1} else {0};
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::CY, test_bit!(s, 15));
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);

        self.reset_prefix();
    }

    fn ror(&mut self) {
        let s = self.regs[self.src];
        let result = (s >> 1) | if self.flags.contains(FXFlags::CY) {bit!(15, u16)} else {0};
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::CY, test_bit!(s, 0));
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);

        self.reset_prefix();
    }
}

// Mult
impl SuperFX {
    fn mult_word(&mut self) {
        let s = (self.regs[self.src] as i16) as i32;
        let op = (self.regs[6] as i16) as i32;
        let result = (s * op) as u32;
        self.flags.set(FXFlags::Z, result == 0);
        //self.flags.set(FXFlags::CY, test_bit!(s, 0)); ??
        self.flags.set(FXFlags::S, test_bit!(result, 31, u32));

        if self.flags.contains(FXFlags::ALT1) { // LMULT
            self.set_dst_reg(hi32!(result));
            self.regs[4] = lo32!(result);
        } else {                                // FMULT
            self.set_dst_reg(hi32!(result));
        }

        self.reset_prefix();
    }

    fn mult_byte(&mut self, n: u8) {
        match self.alt() {
            0 => self.signed_mult(lo!(self.regs[n as usize])),
            1 => self.unsigned_mult(lo!(self.regs[n as usize])),
            2 => self.signed_mult(n),
            3 => self.unsigned_mult(n),
            _ => unimplemented!(),
        }
        self.reset_prefix();
    }

    fn signed_mult(&mut self, op: u8) {
        let s = (self.regs[self.src] as i8) as i16;
        let n = (op as i8) as i16;
        let result = (s * n) as u16;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);
    }

    fn unsigned_mult(&mut self, op: u8) {
        let s = self.regs[self.src] as u16;
        let n = op as u16;
        let result = s * n;
        self.flags.set(FXFlags::Z, result == 0);
        self.flags.set(FXFlags::S, test_bit!(result, 15));
        self.set_dst_reg(result);
    }
}

impl SuperFX {
    fn read_mem(&mut self, bank: u8, addr: u16) -> u8 {
        let data = self.mem.fx_read(bank, addr);
        self.cycle_count += if self.clock_select {5} else {3};
        data
    }

    fn write_mem(&mut self, bank: u8, addr: u16, data: u8) {
        self.mem.fx_write(bank, addr, data);
        self.cycle_count += if self.clock_select {5} else {3};
    }

    fn fetch(&mut self) -> u8 {
        let data = match self.cache.try_read(self.pc) {
            CacheResult::InCache(data) => {
                self.cycle_count += 1;
                data
            },
            CacheResult::Request => {
                let data = self.read_mem(self.pb, self.pc);
                self.cache.fill(self.pc, data);
                data
            },
            CacheResult::OutsideCache => self.read_mem(self.pb, self.pc)
        };
        self.pc = self.regs[PC_REG];
        self.regs[PC_REG] = self.regs[PC_REG].wrapping_add(1);
        data
    }

    fn set_dst_reg(&mut self, data: u16) {
        if self.dst == PC_REG {
            self.set_pc_reg(data);
        } else {
            self.regs[self.dst] = data;
        }
    }

    // Set new PC and setup cache lines.
    fn set_pc_reg(&mut self, data: u16) {
        // Check if we need to fill old line.
        if self.regs[PC_REG] & 0xF != 0 {
            match self.cache.try_read(self.regs[PC_REG]) {
                CacheResult::InCache(_) => {},      // Cache line is already loaded.
                CacheResult::Request => self.fill_cache_line_end(),
                CacheResult::OutsideCache => {},    // We are outisde the cache and don't care.
            }
        }

        self.regs[PC_REG] = data;

        // Check if we need to fill new line.
        if self.regs[PC_REG] & 0xF != 0 {
            match self.cache.try_read(self.regs[PC_REG]) {
                CacheResult::InCache(_) => {},      // Cache line is already loaded.
                CacheResult::Request => self.fill_cache_line_start(),
                CacheResult::OutsideCache => {},    // We are outisde the cache and don't care.
            }
        }
    }

    // Fill to the end of the current cache line.
    // We can assume the data at the current PC is _not_ in the cache.
    fn fill_cache_line_end(&mut self) {
        let pc = self.regs[PC_REG];
        let line_end_addr = (pc & 0xFFF0) + 0x10;
        for i in pc..line_end_addr {
            let data = self.read_mem(self.pb, i);
            self.cache.fill(i, data);
        }
    }

    // Fill the current cache line up to the current PC.
    fn fill_cache_line_start(&mut self) {
        let pc = self.regs[PC_REG];
        let line_start_addr = pc & 0xFFF0;
        for i in line_start_addr..pc {
            let data = self.read_mem(self.pb, i);
            self.cache.fill(i, data);
        }
    }

    fn alt(&self) -> u16 {
        (self.flags & (FXFlags::ALT1 | FXFlags::ALT2)).bits() >> 8
    }
}