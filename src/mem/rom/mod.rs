// ROM types
mod header;
mod sram;

use std::{
    collections::HashMap,
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File
};

use crate::constants::timing;

use sram::*;

const LOROM_RAM_BANK_SIZE: u32 = 0x8000;
const HIROM_RAM_BANK_SIZE: u32 = 0x2000;

const SPEED_BIT: u8 = 0;

pub trait Cart {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize);
    fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize;
    fn flush(&mut self);

    fn set_rom_speed(&mut self, data: u8);

    fn name(&self) -> String;
}

pub fn create_cart(cart_path: &str, save_path: &str) -> Box<dyn Cart> {
    let rom_file = File::open(cart_path).expect(&format!("Couldn't open file {}", cart_path));
    //let rom_size = rom_file.metadata().expect("Couldn't get metadata for file.").len();

    let mut reader = BufReader::new(rom_file);

    let mut header = header::ROMHeader::new();

    if header.try_lo(&mut reader) {
        let sram = create_sram(save_path, header.sram_size()).expect("Couldn't make save file.");
        let name = header.rom_name();

        return if header.rom_size() > (1 << 21) {
            println!("LOROM Large {:X}: {}", header.rom_mapping(), name);
            Box::new(LoROMLarge::new(reader, sram, header.fast_rom(), name))
        } else {
            println!("LOROM {:X}: {}", header.rom_mapping(), name);
            Box::new(LoROM::new(reader, sram, header.fast_rom(), name))
        };
    }

    // Check for HiROM
    if header.try_hi(&mut reader) {
        let sram = create_sram(save_path, header.sram_size()).expect("Couldn't make save file.");
        let name = header.rom_name();
        println!("HIROM {:X}: {}", header.rom_mapping(), name);
        return Box::new(HiROM::new(reader, sram, header.fast_rom(), name));
    } else {
        panic!("Unrecognised ROM: {:X}", header.rom_mapping());
    }
}

// ROM banks.
struct ROM {
    rom_file:   BufReader<File>,
    banks:      HashMap<u8, Vec<u8>>,
    bank_size:  usize
}

impl ROM {
    fn new(cart_file: BufReader<File>, bank_size: usize) -> Self {
        // read and store
        ROM {
            rom_file:   cart_file,
            banks:      HashMap::new(),
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
    rom:        ROM,
    ram:        Box<dyn SRAM>,

    fast_rom:   bool,
    rom_speed:  usize,

    name:       String
}

impl LoROM {
    pub fn new(cart_file: BufReader<File>, sram: Box<dyn SRAM>, fast: bool, name: String) -> Self {
        LoROM {
            rom:        ROM::new(cart_file, 0x8000),
            ram:        sram,

            fast_rom:   fast,
            rom_speed:  timing::SLOW_MEM_ACCESS,

            name:       name,
        }
    }
}

impl Cart for LoROM {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize) {
        let internal_bank = bank % 0x80;

        /*if addr >= 0x8000 {
            (self.rom.read(internal_bank % 0x40, addr % 0x8000), self.rom_speed)
        } else if internal_bank >= 0x70 {
            let ram_bank = ((internal_bank - 0x70) as u32) * LOROM_RAM_BANK_SIZE;
            (self.ram.read(ram_bank + addr as u32), timing::SLOW_MEM_ACCESS)
        } else {
            (0, timing::SLOW_MEM_ACCESS)
        }*/
        match internal_bank {
            0x00..=0x3F if addr >= 0x8000 => (self.rom.read(internal_bank, addr % 0x8000), self.rom_speed),
            0x70..=0x7F => {
                let ram_bank = ((internal_bank - 0x70) as u32) * LOROM_RAM_BANK_SIZE;
                (self.ram.read(ram_bank + addr as u32), timing::SLOW_MEM_ACCESS)
            },
            _ => (self.rom.read(internal_bank % 0x40, addr % 0x8000), self.rom_speed),
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

    fn flush(&mut self) {
        self.ram.flush();
    }

    fn set_rom_speed(&mut self, data: u8) {
        self.rom_speed = if self.fast_rom && test_bit!(data, SPEED_BIT, u8) {
            timing::FAST_MEM_ACCESS
        } else {
            timing::SLOW_MEM_ACCESS
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

// For LoROM carts that are larger than 2MB.
pub struct LoROMLarge {
    rom:        ROM,
    ram:        Box<dyn SRAM>,

    fast_rom:   bool,
    rom_speed:  usize,

    name:       String
}

impl LoROMLarge {
    pub fn new(cart_file: BufReader<File>, sram: Box<dyn SRAM>, fast: bool, name: String) -> Self {
        LoROMLarge {
            rom:        ROM::new(cart_file, 0x8000),
            ram:        sram,

            fast_rom:   fast,
            rom_speed:  timing::SLOW_MEM_ACCESS,

            name:       name,
        }
    }
}

impl Cart for LoROMLarge {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize) {
        let internal_bank = bank % 0x80;

        match internal_bank {
            0x00..=0x3F if addr >= 0x8000 => (self.rom.read(internal_bank, addr % 0x8000), self.rom_speed),
            0x70..=0x7F => {
                let ram_bank = ((internal_bank - 0x70) as u32) * LOROM_RAM_BANK_SIZE;
                (self.ram.read(ram_bank + addr as u32), timing::SLOW_MEM_ACCESS)
            },
            _ => (self.rom.read(internal_bank, addr % 0x8000), self.rom_speed),
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

    fn flush(&mut self) {
        self.ram.flush();
    }

    fn set_rom_speed(&mut self, data: u8) {
        self.rom_speed = if self.fast_rom && test_bit!(data, SPEED_BIT, u8) {
            timing::FAST_MEM_ACCESS
        } else {
            timing::SLOW_MEM_ACCESS
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

pub struct HiROM {
    rom:        ROM,
    ram:        Box<dyn SRAM>,

    fast_rom:   bool,
    rom_speed:  usize,

    name:       String
}

impl HiROM {
    pub fn new(cart_file: BufReader<File>, sram: Box<dyn SRAM>, fast: bool, name: String) -> Self {
        HiROM {
            rom:        ROM::new(cart_file, 0x10000),
            ram:        sram,

            fast_rom:   fast,
            rom_speed:  timing::SLOW_MEM_ACCESS,

            name:       name,
        }
    }
}

impl Cart for HiROM {
    fn read(&mut self, bank: u8, addr: u16) -> (u8, usize) {
        let internal_bank = bank % 0x80;

        match internal_bank {
            0x00..=0x3F if addr >= 0x8000 => (self.rom.read(internal_bank, addr), self.rom_speed),
            0x20..=0x2F if addr >= 0x6000 => {
                let ram_bank = ((internal_bank - 0x20) as u32) * HIROM_RAM_BANK_SIZE;
                (self.ram.read(ram_bank + (addr as u32 - 0x6000)), timing::SLOW_MEM_ACCESS)
            },
            0x30..=0x3F if addr >= 0x6000 => {
                let ram_bank = ((internal_bank - 0x30) as u32) * HIROM_RAM_BANK_SIZE;
                (self.ram.read(ram_bank + (addr as u32 - 0x6000)), timing::SLOW_MEM_ACCESS)
            },
            0x40..=0x7F => (self.rom.read(internal_bank % 0x40, addr), self.rom_speed),
            _ => (0, timing::SLOW_MEM_ACCESS)
        }
    }

    fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize {
        let internal_bank = bank % 0x80;

        match internal_bank {
            0x20..=0x2F if addr >= 0x6000 => {
                let ram_bank = ((internal_bank - 0x20) as u32) * HIROM_RAM_BANK_SIZE;
                self.ram.write(ram_bank + (addr as u32 - 0x6000), data);
            },
            0x30..=0x3F if addr >= 0x6000 => {
                let ram_bank = ((internal_bank - 0x30) as u32) * HIROM_RAM_BANK_SIZE;
                self.ram.write(ram_bank + (addr as u32 - 0x6000), data);
            },
            _ => {}
        }

        timing::SLOW_MEM_ACCESS
    }

    fn flush(&mut self) {
        self.ram.flush();
    }

    fn set_rom_speed(&mut self, data: u8) {
        self.rom_speed = if self.fast_rom && test_bit!(data, SPEED_BIT, u8) {
            timing::FAST_MEM_ACCESS
        } else {
            timing::SLOW_MEM_ACCESS
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}