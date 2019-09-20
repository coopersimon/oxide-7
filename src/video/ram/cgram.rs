// CGRAM: contains palette information.

pub struct CGRAM {
    data:       Vec<u8>,
    addr:       u8,
    hi_byte:    bool,
    buffer:     u8,

    bg_dirty:   bool,   // If any byte is changed, this is set.
    obj_dirty:  bool,   // If the top 256 bytes are changed, this is set.
}

impl CGRAM {
    pub fn new() -> Self {
        CGRAM {
            data:       vec![0; 512],
            addr:       0,
            hi_byte:    false,
            buffer:     0,

            bg_dirty:   true,
            obj_dirty:  true,
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

            if addr >= 256 {
                self.obj_dirty = true;
            }
            self.bg_dirty = true;

            self.data[addr] = self.buffer;
            self.data[addr + 1] = data;

            self.addr = self.addr.wrapping_add(1);
            self.hi_byte = false;
        } else {
            self.buffer = data;
            self.hi_byte = true;
        }
    }

    // For use by renderer memory caches.
    pub fn ref_data<'a>(&'a mut self) -> &'a [u8] {
        &self.data
    }

    pub fn is_bg_dirty(&self) -> bool {
        self.bg_dirty
    }

    pub fn is_obj_dirty(&self) -> bool {
        self.obj_dirty
    }

    pub fn reset_dirty(&mut self) {
        self.bg_dirty = false;
        self.obj_dirty = false;
    }
}