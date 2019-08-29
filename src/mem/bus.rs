// Address Buses A and B
use super::{
    RAM,
    rom::{
        Cart,
        LoROM
    }
};

// Timings
const FAST_MEM_ACCESS: usize = 6;
const SLOW_MEM_ACCESS: usize = 8;
const XSLOW_MEM_ACCESS: usize = 12;

// A bus to attach to the CPU (Address Bus A).
pub struct MemBus {
    wram:   RAM,
    bus_b:  AddrBusB,
    cart:   Box<dyn Cart>
}

impl MemBus {
    pub fn new(cart_path: &str) -> Self {
        // Open file and check type...

        MemBus {
            wram: RAM::new(0x20000),
            bus_b: AddrBusB::new(),
            cart: Box::new(LoROM::new())
        }
    }

    pub fn read(&mut self, addr: u32) -> (u8, usize) {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => (self.wram.read(offset as u32), SLOW_MEM_ACCESS),
                0x2100..=0x21FF => (self.bus_b.read(lo!(offset)), FAST_MEM_ACCESS),
                0x3000..=0x3FFF => (0, FAST_MEM_ACCESS), // Extensions
                0x4000..=0x40FF => (0, XSLOW_MEM_ACCESS), // Joypads
                0x4200..=0x44FF => (0, FAST_MEM_ACCESS), // DMA
                0x6000..=0xFFFF => (self.cart.read(bank, offset), SLOW_MEM_ACCESS),
                _               => (0, FAST_MEM_ACCESS),  // Unmapped
            },
            0x40..=0x7D | 0xC0..=0xFF => (self.cart.read(bank, offset), SLOW_MEM_ACCESS),
            0x7E | 0x7F => (self.wram.read(addr - 0x7E0000), SLOW_MEM_ACCESS),
        }
    }

    pub fn write(&mut self, addr: u32, data: u8) -> usize {
        let bank = hi24!(addr);
        let offset = lo24!(addr);

        match bank {
            0x00..=0x3F | 0x80..=0xBF => match offset {
                0x0000..=0x1FFF => {self.wram.write(offset as u32, data); SLOW_MEM_ACCESS},
                0x2100..=0x21FF => {self.bus_b.write(lo!(offset), data); FAST_MEM_ACCESS},
                0x3000..=0x3FFF => FAST_MEM_ACCESS, // Extensions
                0x4000..=0x40FF => XSLOW_MEM_ACCESS, // Joypads
                0x4200..=0x44FF => FAST_MEM_ACCESS, // DMA
                0x6000..=0xFFFF => {self.cart.write(bank, offset, data); SLOW_MEM_ACCESS},
                _               => FAST_MEM_ACCESS,  // Unmapped
            },
            0x40..=0x7D | 0xC0..=0xFF => {self.cart.write(bank, offset, data); SLOW_MEM_ACCESS},
            0x7E | 0x7F => {self.wram.write(addr - 0x7E0000, data); SLOW_MEM_ACCESS},
        }
    }
}

// Address Bus B, used for hardware registers.
struct AddrBusB {

}

impl AddrBusB {
    fn new() -> Self {
        AddrBusB {

        }
    }

    pub fn read(&self, addr: u8) -> u8 {
        0
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        
    }
}