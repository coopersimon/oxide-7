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

const LOROM_RAM_BANK_SIZE: u32 = 0x8000;
const HIROM_RAM_BANK_SIZE: u32 = 0x2000;

const SPEED_BIT: u8 = 0;

pub trait Cart {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize);
    fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize;

    fn set_write_speed(&mut self, data: u8);
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
                .expect(&format!("Couldn't swap in bank at pos ${:X}-${:X}, bank: ${:X}, addr: ${:X}", offset, offset + self.bank_size as u64, bank, addr));

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
    pub fn new(cart_file: BufReader<File>/*, fast: bool*/) -> Self {
        LoROM {
            rom: ROM::new(cart_file, 0x8000),
            ram: RAM::new(512 * 1024),

            rom_speed: timing::SLOW_MEM_ACCESS
        }
    }
}

impl Cart for LoROM {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize) {
        let internal_bank = bank % 0x80;

        if addr >= 0x8000 {
            (self.rom.read(internal_bank, addr % 0x8000), if bank < 0x80 {timing::SLOW_MEM_ACCESS} else {self.rom_speed})
        } else if internal_bank >= 0x70 {
            let ram_bank = ((internal_bank - 0x70) as u32) * LOROM_RAM_BANK_SIZE;
            (self.ram.read(ram_bank + addr as u32), timing::SLOW_MEM_ACCESS)
        } else {
            (0, timing::SLOW_MEM_ACCESS)
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize {
        let internal_bank = bank % 0x80;

        if internal_bank >= 0x70 {
            let ram_bank = ((internal_bank - 0x70) as u32) * LOROM_RAM_BANK_SIZE;
            self.ram.write(ram_bank + addr as u32, data);
        }

        timing::SLOW_MEM_ACCESS
    }

    fn set_write_speed(&mut self, data: u8) {
        self.rom_speed = if test_bit!(data, SPEED_BIT, u8) {
            timing::FAST_MEM_ACCESS
        } else {
            timing::SLOW_MEM_ACCESS
        }
    }
}

pub struct HiROM {
    rom: ROM,
    ram: RAM,

    rom_speed: usize,
}

impl HiROM {
    pub fn new(cart_file: BufReader<File>/*, fast: bool*/) -> Self {
        HiROM {
            rom: ROM::new(cart_file, 0x10000),
            ram: RAM::new(256 * 1024),

            rom_speed: timing::SLOW_MEM_ACCESS
        }
    }
}

impl Cart for HiROM {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize) {
        let internal_bank = bank % 0x80;

        match internal_bank {
            0x00..=0x3F if addr >= 0x8000 => (self.rom.read(internal_bank, addr), if bank < 0x80 {timing::SLOW_MEM_ACCESS} else {self.rom_speed}),
            0x20..=0x3F if addr >= 0x6000 => {
                let ram_bank = ((internal_bank - 0x20) as u32) * HIROM_RAM_BANK_SIZE;
                (self.ram.read(ram_bank + (addr as u32 - 0x6000)), timing::SLOW_MEM_ACCESS)
            },
            0x40..=0x7F => (self.rom.read(internal_bank % 0x40, addr), if bank < 0x80 {timing::SLOW_MEM_ACCESS} else {self.rom_speed}),
            _ => (0, timing::SLOW_MEM_ACCESS)
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize {
        let internal_bank = bank % 0x80;

        match internal_bank {
            0x20..=0x3F if addr >= 0x6000 => {
                let ram_bank = ((internal_bank - 0x20) as u32) * HIROM_RAM_BANK_SIZE;
                self.ram.write(ram_bank + (addr as u32 - 0x6000), data);
            },
            _ => {}
        }

        timing::SLOW_MEM_ACCESS
    }

    fn set_write_speed(&mut self, data: u8) {
        self.rom_speed = if test_bit!(data, SPEED_BIT, u8) {
            timing::FAST_MEM_ACCESS
        } else {
            timing::SLOW_MEM_ACCESS
        }
    }
}