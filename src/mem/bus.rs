// Address Buses A and B
use std::{
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File
};

use crate::{
    joypad::{Button, JoypadMem},
    constants::timing::*,
    video::PPU
};

use super::{
    RAM,
    rom::{
        Cart,
        LoROM
    }
};

// A bus to attach to the CPU (Address Bus A).
pub struct MemBus {
    // Devices
    bus_b:      AddrBusB,
    joypads:    JoypadMem,

    // Memory
    cart:       Box<dyn Cart>,
    wram:       RAM,

    // Stored addresses
    wram_addr:  u32,

    // Extensions
}

impl MemBus {
    pub fn new(cart_path: &str) -> Self {
        // Open ROM file.
        let f = File::open(cart_path).expect(&format!("Couldn't open file {}", cart_path));

        let reader = BufReader::new(f);
        
        let cart = MemBus::make_cart(reader);

        MemBus {
            bus_b:      AddrBusB::new(),
            joypads:    JoypadMem::new(),
            
            cart:       cart,
            wram:       RAM::new(0x20000),

            wram_addr:  0,
        }
    }

    pub fn read(&mut self, addr: u32) -> (u8, usize) {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => (self.wram.read(offset as u32), SLOW_MEM_ACCESS),

                0x2134..=0x2143 => (self.bus_b.read(lo!(offset)), FAST_MEM_ACCESS),
                0x2180          => self.read(self.wram_addr),
                0x2100..=0x21FF => (0, FAST_MEM_ACCESS),
                0x3000..=0x3FFF => (0, FAST_MEM_ACCESS),                                // Extensions

                0x4000..=0x4015 |
                0x4000..=0x41FF => (self.joypads.read(offset), XSLOW_MEM_ACCESS),
                0x4200..=0x42FF => (self.read_reg(offset), FAST_MEM_ACCESS),
                0x4300..=0x44FF => (0, FAST_MEM_ACCESS),                                // DMA

                0x6000..=0xFFFF => self.cart.read(bank, offset),
                _               => (0, FAST_MEM_ACCESS),                                // Unmapped
            },
            0x40..=0x7D | 0xC0..=0xFF => self.cart.read(bank, offset),
            0x7E | 0x7F => (self.wram.read(addr - 0x7E0000), SLOW_MEM_ACCESS),
        }
    }

    pub fn write(&mut self, addr: u32, data: u8) -> usize {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => {self.wram.write(offset as u32, data); SLOW_MEM_ACCESS},

                0x2100..=0x2143 => {self.bus_b.write(lo!(offset), data); FAST_MEM_ACCESS},
                0x2180          => self.write(self.wram_addr, data),
                0x2181          => {self.wram_addr = set_lo24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2182          => {self.wram_addr = set_mid24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2183          => {self.wram_addr = set_hi24!(self.wram_addr, data); FAST_MEM_ACCESS},
                0x2100..=0x21FF => FAST_MEM_ACCESS,
                0x3000..=0x3FFF => FAST_MEM_ACCESS, // Extensions

                0x4000..=0x4015 |
                0x4017..=0x41FF => XSLOW_MEM_ACCESS,
                0x4016          => {self.joypads.latch_all(); XSLOW_MEM_ACCESS},

                0x4200..=0x42FF => {self.write_reg(offset, data); FAST_MEM_ACCESS},
                0x4300..=0x44FF => FAST_MEM_ACCESS, // DMA

                0x6000..=0xFFFF => self.cart.write(bank, offset, data),
                _               => FAST_MEM_ACCESS,  // Unmapped
            },
            0x40..=0x7D | 0xC0..=0xFF => self.cart.write(bank, offset, data),
            0x7E | 0x7F => {self.wram.write(addr - 0x7E0000, data); SLOW_MEM_ACCESS},
        }
    }

    pub fn set_joypad_button(&mut self, button: Button, joypad: usize) {
        self.joypads.set_button(button, joypad);
    }
}

// Internal
impl MemBus {
    fn make_cart(mut reader: BufReader<File>) -> Box<dyn Cart> {
        let mut buf = [0; 0x40];
        
        // Check for LoROM
        reader.seek(SeekFrom::Start(0x7FB0)).expect("Couldn't seek to cartridge header.");
        reader.read_exact(&mut buf).expect("Couldn't read cartridge header.");

        if (buf[0x25] & 0x21) == 0x20 {
            return Box::new(LoROM::new(reader, (buf[0x25] & bit!(4)) != 0));
        } else {
            panic!("Unrecognised ROM");
        }

        // Check for HiROM
        //reader.seek(SeekFrom::Start(0xFFB0)).expect("Couldn't seek to cartridge header.");
        //reader.read_exact(&mut buf).expect("Couldn't read cartridge header.");
    }

    // Internal status registers.
    fn read_reg(&mut self, addr: u16) -> u8 {
        match addr {
            0x4212          => self.joypads.is_ready(),
            0x4218..=0x421F => self.joypads.read(addr),
            _ => 0,
        }
    }

    fn write_reg(&mut self, addr: u16, data: u8) {
        match addr {
            0x4200          => self.joypads.enable_counter(data),
            _ => {},
        }
    }
}

// Address Bus B, used for hardware registers.
struct AddrBusB {
    ppu: PPU
}

impl AddrBusB {
    fn new() -> Self {
        AddrBusB {
            ppu: PPU::new()
        }
    }

    fn read(&mut self, addr: u8) -> u8 {
        match addr {
            0x34..=0x3F => self.ppu.read_mem(addr),
            0x40..=0x43 => 0, // APU IO
            _ => unreachable!()
        }
    }

    fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x00..=0x33 => self.ppu.write_mem(addr, data),
            0x37 => {}, // Software latch (?)
            0x40..=0x43 => {}, // APU IO
            _ => unreachable!()
        }
    }
}