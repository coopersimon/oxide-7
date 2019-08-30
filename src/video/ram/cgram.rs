// CGRAM: contains palette information.

pub struct CGRAM {
    data: Vec<u8>,
    addr: u8
}

impl CGRAM {
    pub fn new() -> Self {
        CGRAM {
            data: vec![0; 256],
            addr: 0
        }
    }

    pub fn set_addr(&mut self, addr: u8) {
        self.addr = addr;
    }

    pub fn read(&mut self) -> u8 {
        let ret = self.data[self.addr as usize];
        self.addr = self.addr.wrapping_add(1);
        ret
    }

    pub fn write(&mut self, data: u8) {
        self.data[self.addr as usize] = data;
        self.addr = self.addr.wrapping_add(1);
    }

    // For use by renderer memory caches.
    pub fn ref_data<'a>(&'a self) -> &'a [u8] {
        &self.data
    }
}