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

use crate::{
    constants::timing,
    expansion::*
};

use header::*;
use sram::*;

const LOROM_RAM_BANK_SIZE: u32 = 0x8000;
const HIROM_RAM_BANK_SIZE: u32 = 0x2000;

const SPEED_BIT: u8 = 0;

pub fn create_cart(cart_path: &str, save_path: &str, dsp_path: Option<&str>) -> Box<Cart> {
    let rom_file = File::open(cart_path).expect(&format!("Couldn't open file {}", cart_path));
    //let rom_size = rom_file.metadata().expect("Couldn't get metadata for file.").len();

    let mut reader = BufReader::new(rom_file);
    let mut header = ROMHeader::new();

    if header.try_lo(&mut reader) {
        let sram = create_sram(save_path, header.sram_size()).expect("Couldn't make save file.");
        let name = header.rom_name();

        let cart = if header.rom_size() > (1 << 21) {
            println!("LOROM Large {:X}: {}", header.rom_mapping(), name);
            Cart::new_lorom_large(reader, sram)
        } else {
            println!("LOROM {:X}: {}", header.rom_mapping(), name);
            Cart::new_lorom(reader, sram)
        }.named(name).fast_rom(header.fast_rom());

        return match header.rom_type().enhancement_chip() {
            Some(EnhancementChip::DSP) => {
                let dsp_path = dsp_path.expect("Must specify DSP ROM path!");
                let dsp_rom_file = File::open(dsp_path).expect(&format!("Couldn't open DSP ROM file {}", dsp_path));
                let mut dsp_reader = BufReader::new(dsp_rom_file);
                let mut buffer = vec![0; 0x2000];
                dsp_reader.read_exact(&mut buffer).expect("Couldn't read into DSP ROM");
                cart.with_dsp_lo(Box::new(DSP::new(&buffer)))
            },
            Some(e) => panic!("Unsupported enhancement chip {:?}", e),
            None => cart,
        }.build();
    }

    // Check for HiROM
    if header.try_hi(&mut reader) {
        let sram = create_sram(save_path, header.sram_size()).expect("Couldn't make save file.");
        let name = header.rom_name();
        println!("HIROM {:X}: {}", header.rom_mapping(), name);
        let cart = Cart::new_hirom(reader, sram)
            .named(name)
            .fast_rom(header.fast_rom());

        match header.rom_type().enhancement_chip() {
            Some(EnhancementChip::DSP) => {
                let dsp_path = dsp_path.expect("Must specify DSP ROM path!");
                let dsp_rom_file = File::open(dsp_path).expect(&format!("Couldn't open DSP ROM file {}", dsp_path));
                let mut dsp_reader = BufReader::new(dsp_rom_file);
                let mut buffer = vec![0; 0x2000];
                dsp_reader.read_exact(&mut buffer).expect("Couldn't read into DSP ROM");
                cart.with_dsp_hi(Box::new(DSP::new(&buffer)))
            },
            Some(e) => panic!("Unsupported enhancement chip {:?}", e),
            None => cart,
        }.build()
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

enum CartDevice {
    ROM(u8, u16),
    RAM(u32),
    Expansion(u32)
}

type CartMappingFn = fn(u8, u16) -> CartDevice;

struct CartMapping {
    start_bank:     u8,
    end_bank:       u8,
    start_addr:     u16,

    addr_mapping:   CartMappingFn
}

struct CartBuilder {
    mappings:   Vec<CartMapping>,

    rom:        Option<ROM>,
    ram:        Option<Box<dyn SRAM>>,
    expansion:  Option<Box<dyn Expansion>>,

    fast_rom:   bool,

    name:       String
}

impl CartBuilder {
    fn new() -> Self {
        CartBuilder {
            mappings:   Vec::new(),
            rom:        None,
            ram:        None,
            expansion:  None,

            fast_rom:   false,

            name:       String::new()
        }
    }

    fn named(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    fn fast_rom(mut self, fast: bool) -> Self {
        self.fast_rom = fast;
        self
    }

    fn with_dsp_lo(mut self, dsp: Box<dyn Expansion>) -> Self {
        self.expansion = Some(dsp);

        self.mappings.insert(0, CartMapping {
            start_bank: 0x30,
            end_bank:   0x3F,
            start_addr: 0x8000,

            addr_mapping: |_, addr| {
                let out_addr = if addr < 0xC000 {0} else {1};
                CartDevice::Expansion(out_addr)
            },
        });

        self
    }

    fn with_dsp_hi(mut self, dsp: Box<dyn Expansion>) -> Self {
        self.expansion = Some(dsp);

        self.mappings.push(CartMapping {
            start_bank: 0x00,
            end_bank:   0x1F,
            start_addr: 0x6000,

            addr_mapping: |_, addr| {
                let out_addr = if addr < 0x7000 {0} else {1};
                CartDevice::Expansion(out_addr)
            },
        });

        self
    }

    fn build(self) -> Box<Cart> {
        Box::new(Cart {
            mappings:   self.mappings,
            rom:        self.rom.expect("Cannot construct cart without ROM"),
            ram:        self.ram.expect("Cannot construct cart without RAM"),
            expansion:  self.expansion,

            fast_rom:   self.fast_rom,
            rom_speed:  timing::SLOW_MEM_ACCESS,

            name:       self.name,
        })
    }
}

pub struct Cart {
    mappings:   Vec<CartMapping>,

    rom:        ROM,
    ram:        Box<dyn SRAM>,
    expansion:  Option<Box<dyn Expansion>>,

    fast_rom:   bool,
    rom_speed:  usize,

    name:       String
}

impl Cart {
    fn new_lorom(cart_file: BufReader<File>, ram: Box<dyn SRAM>) -> CartBuilder {
        let mut builder = CartBuilder::new();
        builder.rom = Some(ROM::new(cart_file, 0x8000));
        builder.ram = Some(ram);

        builder.mappings.push(CartMapping {
            start_bank: 0x00,
            end_bank:   0x3F,
            start_addr: 0x8000,

            addr_mapping: |bank, addr| {
                CartDevice::ROM(bank, addr % 0x8000)
            },
        });

        builder.mappings.push(CartMapping {
            start_bank: 0x40,
            end_bank:   0x6F,
            start_addr: 0,

            addr_mapping: |bank, addr| {
                CartDevice::ROM(bank % 0x40, addr % 0x8000)
            },
        });

        builder.mappings.push(CartMapping {
            start_bank: 0x70,
            end_bank:   0x7F,
            start_addr: 0,

            addr_mapping: |bank, addr| {
                let ram_bank = ((bank - 0x70) as u32) * LOROM_RAM_BANK_SIZE;
                CartDevice::RAM(ram_bank + addr as u32)
            },
        });

        builder
    }

    fn new_lorom_large(cart_file: BufReader<File>, ram: Box<dyn SRAM>) -> CartBuilder {
        let mut builder = CartBuilder::new();
        builder.rom = Some(ROM::new(cart_file, 0x8000));
        builder.ram = Some(ram);

        builder.mappings.push(CartMapping {
            start_bank: 0x00,
            end_bank:   0x3F,
            start_addr: 0x8000,

            addr_mapping: |bank, addr| {
                CartDevice::ROM(bank, addr % 0x8000)
            },
        });

        builder.mappings.push(CartMapping {
            start_bank: 0x40,
            end_bank:   0x6F,
            start_addr: 0,

            addr_mapping: |bank, addr| {
                CartDevice::ROM(bank, addr % 0x8000)
            },
        });

        builder.mappings.push(CartMapping {
            start_bank: 0x70,
            end_bank:   0x7F,
            start_addr: 0,

            addr_mapping: |bank, addr| {
                let ram_bank = ((bank - 0x70) as u32) * LOROM_RAM_BANK_SIZE;
                CartDevice::RAM(ram_bank + addr as u32)
            },
        });

        builder
    }

    fn new_hirom(cart_file: BufReader<File>, ram: Box<dyn SRAM>) -> CartBuilder {
        let mut builder = CartBuilder::new();
        builder.rom = Some(ROM::new(cart_file, 0x10000));
        builder.ram = Some(ram);

        builder.mappings.push(CartMapping {
            start_bank: 0x00,
            end_bank:   0x3F,
            start_addr: 0x8000,

            addr_mapping: |bank, addr| {
                CartDevice::ROM(bank, addr)
            },
        });

        builder.mappings.push(CartMapping {
            start_bank: 0x20,
            end_bank:   0x3F,
            start_addr: 0x6000,

            addr_mapping: |bank, addr| {
                let ram_bank = ((bank % 0x10) as u32) * HIROM_RAM_BANK_SIZE;
                CartDevice::RAM(ram_bank + (addr as u32 - 0x6000))
            },
        });

        builder.mappings.push(CartMapping {
            start_bank: 0x40,
            end_bank:   0x7F,
            start_addr: 0,

            addr_mapping: |bank, addr| {
                CartDevice::ROM(bank % 0x40, addr)
            },
        });

        builder
    }
}

impl Cart {
    pub fn read(&mut self, bank: u8, addr: u16) -> (u8, usize) {
        let internal_bank = bank % 0x80;

        for mapping in self.mappings.iter() {
            if (internal_bank >= mapping.start_bank) &&
                (internal_bank <= mapping.end_bank) &&
                (addr >= mapping.start_addr) {
                return match (mapping.addr_mapping)(internal_bank, addr) {
                    CartDevice::ROM(bank, addr) => (self.rom.read(bank, addr), self.rom_speed),
                    CartDevice::RAM(addr) => (self.ram.read(addr), timing::SLOW_MEM_ACCESS),
                    CartDevice::Expansion(addr) => (self.expansion.as_mut().map_or(0, |e| e.read(addr)), timing::SLOW_MEM_ACCESS),
                };
            }
        }

        (0, timing::SLOW_MEM_ACCESS)
    }

    pub fn write(&mut self, bank: u8, addr: u16, data: u8) -> usize {
        let internal_bank = bank % 0x80;

        for mapping in self.mappings.iter() {
            if (internal_bank >= mapping.start_bank) &&
                (internal_bank <= mapping.end_bank) &&
                (addr >= mapping.start_addr) {
                match (mapping.addr_mapping)(internal_bank, addr) {
                    CartDevice::ROM(_,_) => {},
                    CartDevice::RAM(addr) => self.ram.write(addr, data),
                    CartDevice::Expansion(addr) => self.expansion.as_mut().map_or((), |e| e.write(addr, data)),
                }
            }
        }

        timing::SLOW_MEM_ACCESS
    }

    pub fn flush(&mut self) {
        self.ram.flush();
    }

    pub fn set_rom_speed(&mut self, data: u8) {
        self.rom_speed = if self.fast_rom && test_bit!(data, SPEED_BIT, u8) {
            timing::FAST_MEM_ACCESS
        } else {
            timing::SLOW_MEM_ACCESS
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn clock(&mut self, cycles: usize) {
        if let Some(ex) = self.expansion.as_mut() {
            ex.clock(cycles);
        }
    }
}
