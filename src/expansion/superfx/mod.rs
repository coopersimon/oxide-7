// MARIO Chip, Super FX and Super FX 2 (GSU)
mod mem;

use bitflags::bitflags;

use crate::{
    common::Interrupt,
    mem::rom::{ROM, SRAM}
};

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

const PC_REG: usize = 15;

pub struct SuperFX {
    regs:       [u16; 16],
    regs_latch: u8,

    flags:      FXFlags,
    pb:         u8,
    romb:       u8,
    ramb:       u8,
    cbr:        u16,
    //bram:       u8,
    cfg:        Config,

    src:        usize,
    dst:        usize,

    // TODO: move?
    screen_base:    u8,
    screen_mode:    ScreenMode,
    colr:           u8,
    por:            PlotOption,

    version:        u8,
    clock_select:   bool,

    cache:          [u8; 0x200],
    mem:            FXMem,
}

impl SuperFX {
    pub fn new(rom: ROM, sram: Box<dyn SRAM>) -> Self {
        SuperFX {
            regs:       [0; 16],
            regs_latch: 0,

            flags:      FXFlags::default(),
            pb:         0,
            romb:       0,
            ramb:       0,
            cbr:        0,
            //bram:       0,
            cfg:        Config::default(),

            src:        0,
            dst:        0,

            screen_base:    0,
            screen_mode:    ScreenMode::default(),
            colr:           0,
            por:            PlotOption::default(),

            version:        0,
            clock_select:   false,

            cache:          [0; 0x200],
            mem:            FXMem::new(rom, sram),
        }
    }

    fn step(&mut self) {
        
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
        // TODO
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
            0x303E => lo!(self.cbr),
            0x303F => hi!(self.cbr),

            0x3100..=0x32FF => self.cache[(addr - 0x3100) as usize],
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

            0x3100..=0x32FF => self.cache[(addr - 0x3100) as usize] = data,
            _ => {},
        }
        
        
    }

    fn set_status_flags(&mut self, data: u8) {
        self.flags.set(FXFlags::Z, test_bit!(data, 1, u8));
        self.flags.set(FXFlags::CY, test_bit!(data, 2, u8));
        self.flags.set(FXFlags::S, test_bit!(data, 3, u8));
        self.flags.set(FXFlags::OV, test_bit!(data, 4, u8));
        self.flags.set(FXFlags::GO, test_bit!(data, 5, u8));    // TODO: start/stop
    }
}

impl SuperFX {
    fn fetch(&mut self) -> u8 {
        let data = self.mem.fx_read(self.pb, self.regs[PC_REG]);
        self.regs[PC_REG] = self.regs[PC_REG].wrapping_add(1);
        data
    }
}