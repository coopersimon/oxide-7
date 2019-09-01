// Memory
mod bus;
mod dma;
mod rom;

pub use bus::MemBus;

// Random access memory.
pub struct RAM {
    data: Vec<u8>
}

impl RAM {
    pub fn new(size: usize) -> Self {
        RAM {
            data: vec![0; size]
        }
    }

    pub fn read(&self, addr: u32) -> u8 {
        self.data[addr as usize]
    }

    pub fn write(&mut self, addr: u32, data: u8) {
        self.data[addr as usize] = data;
    }
}