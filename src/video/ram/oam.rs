// OAM (Object Attribute Memory), contains sprite info

const LO_TABLE_SIZE: usize = 512;
const HI_TABLE_SIZE: usize = 32;

pub struct OAM {
    lo_table:   Vec<u8>,
    hi_table:   Vec<u8>,

    addr_lo:    u8, // Cached internal address values
    addr_hi:    u8,

    addr:       usize,
    hi_byte:    bool,
    buffer:     u8,

    dirty:      bool
}

impl OAM {
    pub fn new() -> Self {
        OAM {
            lo_table:   vec![0; LO_TABLE_SIZE],
            hi_table:   vec![0; HI_TABLE_SIZE],

            addr_lo:    0,
            addr_hi:    0,

            addr:       0,
            hi_byte:    false,
            buffer:     0,

            dirty:      true,
        }
    }

    pub fn set_addr_lo(&mut self, addr: u8) {
        self.addr_lo = addr;
        self.set_addr();
    }

    pub fn set_addr_hi(&mut self, addr: u8) {
        self.addr_hi = addr;
        self.set_addr();
    }

    pub fn read(&mut self) -> u8 {
        let addr = self.addr + (if self.hi_byte {1} else {0});

        let ret = if self.addr >= LO_TABLE_SIZE {
            let hi_addr = addr % 32;
            self.hi_table[hi_addr]
        } else {
            self.lo_table[addr]
        };

        self.hi_byte = if self.hi_byte {
            self.addr += 2;
            false
        } else {
            true
        };

        ret
    }

    pub fn write(&mut self, data: u8) {
        // Hi table
        if self.addr >= LO_TABLE_SIZE {
            let addr = self.addr % 32;
            if self.hi_byte {
                self.hi_table[addr + 1] = data;
                
                self.addr += 2;
                self.hi_byte = false;
            } else {
                self.hi_table[addr] = data;
                self.hi_byte = true;
            }
        } else {
            if self.hi_byte {
                self.lo_table[self.addr] = self.buffer;
                self.lo_table[self.addr + 1] = data;

                self.addr += 2;
                self.hi_byte = false;
            } else {
                self.buffer = data;
                self.hi_byte = true;
            }
        }

        self.dirty = true;
    }

    pub fn reset(&mut self) {
        // TODO: should this be called in f-blank?
        self.set_addr();  // TODO: re-enable
    }

    // For use by renderer memory caches.
    pub fn ref_data<'a>(&'a mut self) -> (&'a [u8], &'a [u8]) {
        self.dirty = false;
        (&self.hi_table, &self.lo_table)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl OAM {
    #[inline]
    fn set_addr(&mut self) {
        self.addr = make16!(self.addr_hi, self.addr_lo) as usize;
        self.hi_byte = false;
    }
}