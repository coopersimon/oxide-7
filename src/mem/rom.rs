// ROM types
use std::collections::BTreeMap;

use super::RAM;

pub trait Cart {
    fn read(&mut self, bank: u8, addr: u16) -> u8;
    fn write(&mut self, bank: u8, addr: u16, data: u8);
}

// ROM banks.
struct ROM {
    banks: BTreeMap<u8, Vec<u8>>,
    bank_size: usize
}

impl ROM {
    fn new(file_name: &str, bank_size: usize) -> Self {
        // read and store
        ROM {
            banks: BTreeMap::new(),
            bank_size: bank_size
        }
    }

    fn read(&mut self, bank: u8, addr: u16) -> u8 {
        if let Some(data) = self.banks.get(&bank) {
            data[addr as usize]
        } else {
            // Read
            let data = vec![0; self.bank_size];
            let ret = data[addr as usize];
            self.banks.insert(bank, data);
            ret
        }
    }
}

pub struct LoROM {
    rom: ROM,
    ram: RAM
}

impl LoROM {
    pub fn new() -> Self {
        LoROM {
            rom: ROM::new("", 0x8000),
            ram: RAM::new(512 * 1024)
        }
    }
}

impl Cart for LoROM {
    fn read(&mut self, bank: u8, addr: u16) -> u8 {
        let internal_bank = bank % 0x80;
        if addr >= 0x8000 {
            self.rom.read(internal_bank, addr % 0x8000)
        } else if internal_bank >= 0x70 {
            let ram_bank = ((internal_bank - 0x70) as u32) * 0x8000;
            self.ram.read(ram_bank + addr as u32)
        } else {
            0
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) {
        let internal_bank = bank % 0x80;
        if internal_bank >= 0x70 {
            let ram_bank = ((internal_bank - 0x70) as u32) * 0x8000;
            self.ram.write(ram_bank + addr as u32, data)
        }
    }
}