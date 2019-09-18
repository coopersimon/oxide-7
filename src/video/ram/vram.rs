// VRAM: background maps and pattern data.

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct PortControl: u8 {
        const INC =      bit!(7);
        const REMAP =    bit!(3) | bit!(2); // TODO
        const INC_RATE = bit!(1) | bit!(0);
    }
}

// Address increment rates.
const INC_RATE_1: u8 = 0;
const INC_RATE_32: u8 = 1;

// Size of VRAM (64kB, or 2^16)
const VRAM_SIZE: usize = 64 * 1024;
const MAX: u16 = (VRAM_SIZE - 1) as u16;

pub struct VRAM {
    data:           Vec<u8>,

    port_control:   PortControl,
    byte_addr:      u16,

    dirty_start:    u16,
    dirty_end:      u16
}

impl VRAM {
    pub fn new() -> Self {
        VRAM {
            data:           vec![0; VRAM_SIZE],

            port_control:   PortControl::default(),
            byte_addr:      0,

            dirty_start:    MAX,
            dirty_end:      0
        }
    }

    pub fn set_port_control(&mut self, data: u8) {
        self.port_control = PortControl::from_bits_truncate(data);
    }

    pub fn set_addr_lo(&mut self, addr: u8) {
        let old_word_addr = self.byte_addr / 2;
        let new_word_addr = set_lo!(old_word_addr, addr);
        self.byte_addr = new_word_addr * 2;
    }

    pub fn set_addr_hi(&mut self, addr: u8) {
        let old_word_addr = self.byte_addr / 2;
        let new_word_addr = set_hi!(old_word_addr, addr);
        self.byte_addr = new_word_addr * 2;
    }

    pub fn read_lo(&mut self) -> u8 {
        let ret = self.data[self.byte_addr as usize];

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn read_hi(&mut self) -> u8 {
        let ret = self.data[self.byte_addr.wrapping_add(1) as usize];

        if self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn write_lo(&mut self, data: u8) {
        self.data[self.byte_addr as usize] = data;

        if self.dirty_start > self.byte_addr {
            self.dirty_start = self.byte_addr;
        }

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }

    pub fn write_hi(&mut self, data: u8) {
        self.data[self.byte_addr.wrapping_add(1) as usize] = data;

        if self.dirty_end < self.byte_addr {
            self.dirty_end = self.byte_addr;
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
        // Inc the bytes.
        let inc_rate = match (self.port_control & PortControl::INC_RATE).bits() {
            INC_RATE_1  => 2,
            INC_RATE_32 => 64,
            _           => 256
        };

        self.byte_addr = self.byte_addr.wrapping_add(inc_rate);
    }
}