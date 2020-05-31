// Memory
mod bus;
mod dma;
pub mod rom;

pub use bus::AddrBusA;

use crate::common::Interrupt;

// Memory Bus to attach to CPU.
pub trait MemBus {
    fn read(&mut self, addr: u32) -> (u8, usize);
    fn write(&mut self, addr: u32, data: u8) -> usize;
    fn clock(&mut self, cycles: usize) -> Interrupt;
}

/// Random access memory.
#[derive(Default)]
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

    pub fn iter<'a>(&'a self, start: usize) -> RAMIter<'a> {
        RAMIter {
            ram: self,
            read_head: start
        }
    }
}

/// An iterator over RAM.
/// Use RAM::iter to construct this.
pub struct RAMIter<'a> {
    ram: &'a RAM,
    read_head: usize
}

impl Iterator for RAMIter<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let data = self.ram.data[self.read_head];
        self.read_head = (self.read_head + 1) % self.ram.data.len();
        Some(data)
    }
}
