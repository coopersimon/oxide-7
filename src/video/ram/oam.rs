// OAM (Object Attribute Memory), contains sprite info

use bitflags::bitflags;

const LO_TABLE_SIZE: usize = 512;
const NUM_OBJECTS: usize = 128;

bitflags!{
    #[derive(Default)]
    struct ObjectAttributes: u8 {
        const Y_FLIP        = bit!(7);
        const X_FLIP        = bit!(6);
        const PRIORITY      = bits![5, 4];
        const PALETTE       = bits![3, 2, 1];
        const NAME_TABLE    = bit!(0);
    }
}

pub enum SpritePriority {
    _3,
    _2,
    _1,
    _0
}

impl From<ObjectAttributes> for SpritePriority {
    fn from(val: ObjectAttributes) -> Self {
        const PRI3: u8 = bits![5, 4];
        const PRI2: u8 = bit!(5);
        const PRI1: u8 = bit!(4);
        const PRI0: u8 = 0;
        use SpritePriority::*;
        match (val & ObjectAttributes::PRIORITY).bits() {
            PRI3 => _3,
            PRI2 => _2,
            PRI1 => _1,
            PRI0 => _0,
            _ => unreachable!()
        }
    }
}

// A single object in memory.
#[derive(Clone, Default)]
pub struct Object {
    pub x:          i16,
    pub y:          u8,
    pub tile_num:   u8,
    attrs:          ObjectAttributes,
    pub large:      bool,
}

impl Object {
    // CPU side read/write
    fn write_lo(&mut self, addr: usize, val: u8) {
        match addr {
            0 => self.x = set_lo!(self.x as u16, val) as i16,
            1 => self.y = val,
            2 => self.tile_num = val,
            3 => self.attrs = ObjectAttributes::from_bits_truncate(val),
            _ => unreachable!()
        }
    }

    fn write_hi(&mut self, val: u8) {
        let hi_x = if test_bit!(val, 0, u8) {0xFF} else {0};
        self.x = set_hi!(self.x as u16, hi_x) as i16;
        self.large = test_bit!(val, 1, u8);
    }

    fn read_lo(&self, addr: usize) -> u8 {
        match addr {
            0 => lo!(self.x as u16),
            1 => self.y,
            2 => self.tile_num,
            3 => self.attrs.bits(),
            _ => unreachable!()
        }
    }

    fn read_hi(&self) -> u8 {
        let lo_bit = if test_bit!(self.x as u16, 8) {bit!(0)} else {0};
        let hi_bit = if self.large {bit!(1)} else {0};
        hi_bit | lo_bit
    }

    // Renderer readers
    pub fn x_flip(&self) -> bool {
        self.attrs.contains(ObjectAttributes::X_FLIP)
    }

    pub fn y_flip(&self) -> bool {
        self.attrs.contains(ObjectAttributes::Y_FLIP)
    }

    pub fn priority(&self) -> SpritePriority {
        SpritePriority::from(self.attrs)
    }

    pub fn palette_offset(&self) -> usize {
        let palette_num = (self.attrs & ObjectAttributes::PALETTE).bits() as usize;
        palette_num << 3
    }

    pub fn name_table(&self) -> usize {
        (self.attrs & ObjectAttributes::NAME_TABLE).bits() as usize
    }

    // Calculate the tile number based on pixel values.
    // The pixel values inside the tile can be calculated by %8.
    #[inline]
    pub fn calc_tile_num(&self, x: usize, y: usize) -> usize {
        let x_offset = x / 8;
        let x_val = self.tile_num.wrapping_add(x_offset as u8) & 0xF;
        let y_offset = (y / 8) << 4;
        let y_val = self.tile_num.wrapping_add(y_offset as u8) & 0xF0;
        (y_val | x_val) as usize
    }
}

pub struct OAM {
    objects:    Vec<Object>,

    addr_lo:    u8, // Cached internal address values
    addr_hi:    u8,

    addr:       usize,
    hi_byte:    bool,
    buffer:     u8,
}

impl OAM {
    pub fn new() -> Self {
        OAM {
            objects:    vec![Object::default(); NUM_OBJECTS],

            addr_lo:    0,
            addr_hi:    0,

            addr:       0,
            hi_byte:    false,
            buffer:     0,
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
            self.read_hi_table(hi_addr)
        } else {
            self.read_lo_table(addr)
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
                self.write_hi_table(addr + 1, data);
                
                self.addr += 2;
                self.hi_byte = false;
            } else {
                self.write_hi_table(addr, data);
                self.hi_byte = true;
            }
        } else {
            if self.hi_byte {
                self.write_lo_table(self.addr, self.buffer);
                self.write_lo_table(self.addr + 1, data);

                self.addr += 2;
                self.hi_byte = false;
            } else {
                self.buffer = data;
                self.hi_byte = true;
            }
        }
    }

    pub fn reset(&mut self) {
        self.set_addr();
    }

    // For use by renderer memory caches.
    pub fn ref_data<'a>(&'a self) -> &'a [Object] {
        &self.objects
    }
}

impl OAM {
    #[inline]
    fn set_addr(&mut self) {
        self.addr = make16!(self.addr_hi, self.addr_lo) as usize;
        self.hi_byte = false;
    }

    fn write_lo_table(&mut self, lo_addr: usize, val: u8) {
        self.objects[lo_addr / 4].write_lo(lo_addr % 4, val);
    }

    fn write_hi_table(&mut self, hi_addr: usize, val: u8) {
        let start = (hi_addr & 0x1F) * 4;
        for i in 0..4 {
            let hi_bits = (val >> (i * 2)) & bits![1, 0];
            self.objects[start + i].write_hi(hi_bits);
        }
    }

    fn read_lo_table(&self, lo_addr: usize) -> u8 {
        self.objects[lo_addr / 4].read_lo(lo_addr % 4)
    }

    fn read_hi_table(&self, hi_addr: usize) -> u8 {
        let start = (hi_addr & 0x1F) * 4;
        (0..4).fold(0, |acc, i| {
            let hi_bits = self.objects[start + i].read_hi();
            acc | (hi_bits << (i * 2))
        })
    }
}