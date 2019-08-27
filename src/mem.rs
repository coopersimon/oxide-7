// Memory

// Read/Write Memory location
pub trait MemDevice<A, D> {
    fn read(&self, addr: A) -> D;
    fn write(&mut self, addr: A, data: D);
}

// Bus (TODO)
pub struct MemBus {

}

impl MemBus {
    pub fn new() -> Self {
        MemBus {

        }
    }
}

impl MemDevice<u32, u8> for MemBus {
    fn read(&self, addr: u32) -> u8 {
        0
    }

    fn write(&mut self, addr: u32, data: u8) {
    }
}