use crate::mem::rom::{
    ROM, SRAM
};

pub struct FXMem {
    // Memory
    rom:    ROM,
    sram:   Box<dyn SRAM>,
}

impl FXMem {
    pub fn new(rom: ROM, sram: Box<dyn SRAM>) -> Self {
        FXMem {
            rom:    rom,
            sram:   sram,
        }
    }

    pub fn snes_read(&mut self, bank: u8, addr: u16) -> u8 {
        match bank % 0x80 {
            0x00..=0x3F if addr >= 0x8000 => self.rom.read(bank, addr - 0x8000),
            0x00..=0x3F if addr >= 0x6000 => self.sram.read((addr - 0x6000) as u32),
            0x40..=0x5F => self.read_hi(bank - 0x40, addr),
            0x60..=0x7F => {
                let bank_addr = (((bank as u32) - 0x60) % 0x10) * 0x10000;
                self.sram.read(bank_addr + (addr as u32))
            },
            _ => 0,
        }
    }

    pub fn snes_write(&mut self, bank: u8, addr: u16, data: u8) {
        match bank % 0x80 {
            0x00..=0x3F if addr >= 0x6000 => self.sram.write((addr - 0x6000) as u32, data),
            0x60..=0x7F => {
                let bank_addr = (((bank as u32) - 0x60) % 0x10) * 0x10000;
                self.sram.write(bank_addr + (addr as u32), data)
            },
            _ => {},
        }
    }

    pub fn fx_read(&mut self, bank: u8, addr: u16) -> u8 {
        match bank {
            0x00..=0x3F => self.rom.read(bank, addr % 0x8000),
            0x40..=0x5F => self.read_hi(bank - 0x40, addr),
            0x70..=0x71 => {
                let bank_addr = ((bank as u32) - 0x70) * 0x10000;
                self.sram.read(bank_addr + (addr as u32))
            },
            _ => 0,
        }
    }

    pub fn fx_write(&mut self, bank: u8, addr: u16, data: u8) {
        match bank {
            0x70..=0x71 => {
                let bank_addr = ((bank as u32) - 0x70) * 0x10000;
                self.sram.write(bank_addr + (addr as u32), data);
            },
            _ => {},
        }
    }

    pub fn flush(&mut self) {
        self.sram.flush();
    }
}

impl FXMem {
    fn read_hi(&mut self, bank: u8, addr: u16) -> u8 {
        let mapped_bank = if addr >= 0x8000 {
            (bank * 2) + 1
        } else {
            bank * 2
        };

        self.rom.read(mapped_bank, addr % 0x8000)
    }
}