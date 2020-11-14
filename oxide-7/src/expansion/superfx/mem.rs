use crate::mem::{
    rom::{ROM, SRAM},
    RAM,
};


pub struct FXMem {
    // Memory
    rom:    ROM,
    ram:    RAM,
    sram:   Box<dyn SRAM>,

    // Access flags
    ron:    bool,
    //ran:    bool,
}

impl FXMem {
    pub fn new(rom: ROM, sram: Box<dyn SRAM>) -> Self {
        FXMem {
            rom:    rom,
            ram:    RAM::new(128 * 1024),
            sram:   sram,

            ron:    false
        }
    }

    pub fn set_ron(&mut self, val: bool) {
        self.ron = val;
    }

    pub fn snes_read(&mut self, bank: u8, addr: u16) -> u8 {
        use crate::constants::int;
        const COP_VECTOR: (u16, u16) = (int::COP_VECTOR as u16, (int::COP_VECTOR + 1) as u16);
        const BRK_VECTOR: (u16, u16) = (int::BRK_VECTOR as u16, (int::BRK_VECTOR + 1) as u16);
        const NMI_VECTOR: (u16, u16) = (int::NMI_VECTOR as u16, (int::NMI_VECTOR + 1) as u16);
        const IRQ_VECTOR: (u16, u16) = (int::IRQ_VECTOR as u16, (int::IRQ_VECTOR + 1) as u16);

        match bank % 0x80 {
            0x00 if self.ron && addr == COP_VECTOR.0 => 0x04,
            0x00 if self.ron && addr == COP_VECTOR.1 => 0x01,
            0x00 if self.ron && addr == BRK_VECTOR.0 => 0x00,
            0x00 if self.ron && addr == BRK_VECTOR.1 => 0x01,
            0x00 if self.ron && addr == NMI_VECTOR.0 => 0x08,
            0x00 if self.ron && addr == NMI_VECTOR.1 => 0x01,
            0x00 if self.ron && addr == IRQ_VECTOR.0 => 0x0C,
            0x00 if self.ron && addr == IRQ_VECTOR.1 => 0x01,
            0x00..=0x3F if addr >= 0x8000 => self.rom.read(bank, addr - 0x8000),
            0x00..=0x3F if addr >= 0x6000 => self.ram.read((addr - 0x6000) as u32),
            0x40..=0x5F => self.read_hi(bank - 0x40, addr),
            0x70..=0x71 => {
                let bank_addr = ((bank as u32) % 0x10) * 0x10000;
                self.ram.read(bank_addr + (addr as u32))
            },
            0x78..=0x79 => {
                let bank_addr = ((bank as u32) % 0x8) * 0x10000;
                self.sram.read(bank_addr + (addr as u32))
            },
            _ => panic!("Trying to read from {:X}_{:X}", bank, addr),
        }
    }

    pub fn snes_write(&mut self, bank: u8, addr: u16, data: u8) {
        match bank % 0x80 {
            0x00..=0x3F if addr >= 0x6000 => self.ram.write((addr - 0x6000) as u32, data),
            0x70..=0x71 => {
                let bank_addr = ((bank as u32) % 0x10) * 0x10000;
                self.ram.write(bank_addr + (addr as u32), data)
            },
            0x78..=0x79 => {
                let bank_addr = ((bank as u32) % 0x8) * 0x10000;
                self.sram.write(bank_addr + (addr as u32), data)
            },
            _ => panic!("Trying to write to {:X}_{:X}", bank, addr),
        }
    }

    pub fn fx_read(&mut self, bank: u8, addr: u16) -> u8 {
        match bank {
            0x00..=0x3F => self.rom.read(bank, addr % 0x8000),
            0x40..=0x5F => self.read_hi(bank - 0x40, addr),
            0x70..=0x71 => {
                let bank_addr = ((bank as u32) - 0x70) * 0x10000;
                self.ram.read(bank_addr + (addr as u32))
            },
            _ => 0,
        }
    }

    pub fn fx_write(&mut self, bank: u8, addr: u16, data: u8) {
        match bank {
            0x70..=0x71 => {
                let bank_addr = ((bank as u32) - 0x70) * 0x10000;
                self.ram.write(bank_addr + (addr as u32), data);
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