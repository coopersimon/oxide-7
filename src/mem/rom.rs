// ROM types
use std::{
    collections::BTreeMap,
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File
};

use crate::constants::timing;

use super::RAM;

pub trait Cart {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize);
    fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize;
}

// ROM banks.
struct ROM {
    rom_file:   BufReader<File>,
    banks:      BTreeMap<u8, Vec<u8>>,
    bank_size:  usize
}

impl ROM {
    fn new(cart_file: BufReader<File>, bank_size: usize) -> Self {
        // read and store
        ROM {
            rom_file:   cart_file,
            banks:      BTreeMap::new(),
            bank_size:  bank_size
        }
    }

    fn read(&mut self, bank: u8, addr: u16) -> u8 {
        if let Some(data) = self.banks.get(&bank) {
            data[addr as usize]
        } else {
            let mut buf = vec![0; self.bank_size];

            let offset = (bank as u64) * (self.bank_size as u64);
            self.rom_file.seek(SeekFrom::Start(offset))
                .expect("Couldn't swap in bank");

            self.rom_file.read_exact(&mut buf)
                .expect(&format!("Couldn't swap in bank at pos {:X}-{:X}, bank: {:X}, addr: {:X}", offset, offset + self.bank_size as u64, bank, addr));

            let data = buf[addr as usize];
            self.banks.insert(bank, buf);
            data
        }
    }
}

pub struct LoROM {
    rom: ROM,
    ram: RAM,

    rom_speed: usize,
}

impl LoROM {
    pub fn new(cart_file: BufReader<File>, fast: bool) -> Self {
        LoROM {
            rom: ROM::new(cart_file, 0x8000),
            ram: RAM::new(512 * 1024),

            rom_speed: if fast {timing::FAST_MEM_ACCESS} else {timing::SLOW_MEM_ACCESS}
        }
    }
}

impl Cart for LoROM {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize) {
        let internal_bank = bank % 0x80;

        if addr >= 0x8000 {
            (self.rom.read(internal_bank, addr % 0x8000), if bank >= 0x80 {timing::SLOW_MEM_ACCESS} else {self.rom_speed})
        } else if internal_bank >= 0x70 {
            let ram_bank = ((internal_bank - 0x70) as u32) * 0x8000;
            (self.ram.read(ram_bank + addr as u32), timing::SLOW_MEM_ACCESS)
        } else {
            (0, timing::SLOW_MEM_ACCESS)
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize {
        let internal_bank = bank % 0x80;

        if internal_bank >= 0x70 {
            let ram_bank = ((internal_bank - 0x70) as u32) * 0x8000;
            self.ram.write(ram_bank + addr as u32, data);
            timing::SLOW_MEM_ACCESS
        } else {
            if bank >= 0x80 {timing::SLOW_MEM_ACCESS} else {self.rom_speed}
        }
    }
}