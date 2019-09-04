// OAM (Object Attribute Memory), contains sprite info

use bitflags::bitflags;

bitflags!{
    #[derive(Default)]
    struct OAMFlags: u8 {
        const TABLE_SELECT = bit!(0);
    }
}

pub struct OAM {
    lo_table:   Vec<u8>,
    hi_table:   Vec<u8>,

    addr:       u8,
    hi_byte:    bool,
    flags:      OAMFlags,
    buffer:     u8,

    dirty:      bool
}

impl OAM {
    pub fn new() -> Self {
        OAM {
            lo_table:   vec![0; 512],
            hi_table:   vec![0; 32],

            addr:       0,
            hi_byte:    false,
            flags:      OAMFlags::default(),
            buffer:     0,

            dirty:      true,
        }
    }

    pub fn set_addr_lo(&mut self, addr: u8) {
        self.addr = addr;
        self.hi_byte = false;
    }

    pub fn set_addr_hi(&mut self, addr: u8) {
        self.flags = OAMFlags::from_bits_truncate(addr);
    }

    pub fn read(&mut self) -> u8 {
        let addr = ((self.addr as usize) * 2) + (if self.hi_byte {1} else {0});

        let ret = if self.flags.contains(OAMFlags::TABLE_SELECT) {
            self.hi_table[addr]
        } else {
            self.lo_table[addr]
        };

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

            if self.flags.contains(OAMFlags::TABLE_SELECT) {
                self.hi_table[addr] = self.buffer;
                self.hi_table[addr + 1] = data;
            } else {
                self.lo_table[addr] = self.buffer;
                self.lo_table[addr + 1] = data;
            }

            self.addr = self.addr.wrapping_add(1);
            self.hi_byte = false;
        } else {
            self.buffer = data;
            self.hi_byte = true;
        }

        self.dirty = true;
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