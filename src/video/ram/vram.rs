// VRAM: background maps and pattern data.

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct PortControl: u8 {
        const INC =      bit!(7);
        const REMAP =    bit!(3) | bit!(2);
        const INC_RATE = bit!(1) | bit!(0);
    }
}

// Address increment rates.
const INC_RATE_1: u8 = 0;
const INC_RATE_32: u8 = 1;
const INC_RATE_64: u8 = 2;  // TODO: this might be 128 too.
const INC_RATE_128: u8 = 3;

// Size of VRAM (64kB, or 2^16)
const VRAM_SIZE: usize = 64 * 1024;
const MAX: u16 = (VRAM_SIZE - 1) as u16;

pub struct VRAM {
    data:           Vec<u8>,

    port_control:   PortControl,
    addr:           u16,        // Word address

    dirty_start:    u16,
    dirty_end:      u16
}

impl VRAM {
    pub fn new() -> Self {
        VRAM {
            data:           vec![0; VRAM_SIZE],

            port_control:   PortControl::default(),
            addr:           0,

            dirty_start:    MAX,
            dirty_end:      0
        }
    }

    pub fn set_port_control(&mut self, data: u8) {
        self.port_control = PortControl::from_bits_truncate(data);
    }

    pub fn set_addr_lo(&mut self, addr: u8) {
        self.addr = set_lo!(self.addr, addr);
    }

    pub fn set_addr_hi(&mut self, addr: u8) {
        self.addr = set_hi!(self.addr, addr);
    }

    pub fn read_lo(&mut self) -> u8 {
        let ret = self.data[(self.addr as usize) * 2];

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn read_hi(&mut self) -> u8 {
        let ret = self.data[(self.addr as usize) * 2 + 1];

        if self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn write_lo(&mut self, data: u8) {
        self.data[(self.addr as usize) * 2] = data;

        if self.dirty_start > self.addr {
            self.dirty_start = self.addr;
        }

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }

    pub fn write_hi(&mut self, data: u8) {
        self.data[(self.addr as usize) * 2 + 1] = data;

        if self.dirty_end < self.addr {
            self.dirty_end = self.addr;
        }

        if self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }

    // For use by renderer memory caches.
    pub fn ref_data<'a>(&'a self) -> &'a [u8] {
        &self.data
    }

    // Check if any bytes have been altered.
    pub fn is_dirty(&self) -> bool {
        (self.dirty_start != MAX) || (self.dirty_end != 0)
    }

    // Check if a block of bytes overlaps with the dirty range.
    // TODO: return the overlapped area.
    pub fn dirty_range(&self, start: u16, end: u16) -> bool {
        (start <= self.dirty_end) && (end >= self.dirty_start)
    }

    // Reset the dirty range once reading is finished.
    pub fn reset_dirty_range(&mut self) {
        self.dirty_start = MAX;
        self.dirty_end = 0;
    }
}

// Internal
impl VRAM {
    #[inline]
    fn inc_addr(&mut self) {
        let inc_rate = match (self.port_control & PortControl::INC_RATE).bits() {
            INC_RATE_1   => 1,
            INC_RATE_32  => 32,
            INC_RATE_64  => 64,
            INC_RATE_128 => 128,
            _ => unreachable!()
        };

        self.addr = self.addr.wrapping_add(inc_rate);
    }
}