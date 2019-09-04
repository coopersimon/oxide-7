// CGRAM: contains palette information.

pub struct CGRAM {
    data:       Vec<u8>,
    addr:       u8,
    hi_byte:    bool,
    buffer:     u8,

    dirty:      bool
}

impl CGRAM {
    pub fn new() -> Self {
        CGRAM {
            data:       vec![0; 512],
            addr:       0,
            hi_byte:    false,
            buffer:     0,

            dirty:      true
        }
    }

    pub fn set_addr(&mut self, addr: u8) {
        self.addr = addr;
        self.hi_byte = false;
    }

    pub fn read(&mut self) -> u8 {
        let addr = ((self.addr as usize) * 2) + (if self.hi_byte {1} else {0});

        let ret = self.data[addr];

        self.hi_byte = if self.hi_byte {
            self.addr = self.addr.wrapping_add(1);
            false
        } else {
            true
        };

        ret
    }

    pub fn write(&mut self, data: u8) {
        if self.hi_byte {
            let addr = (self.addr as usize) * 2;

            self.data[addr] = self.buffer;
            self.data[addr + 1] = data;

            self.addr = self.addr.wrapping_add(1);
            self.hi_byte = false;
        } else {
            self.buffer = data;
            self.hi_byte = true;
        }

        self.dirty = true;
    }

    // For use by renderer memory caches.
    pub fn ref_data<'a>(&'a mut self) -> &'a [u8] {
        self.dirty = false;
        &self.data
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}