// VRAM: background maps and pattern data.

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct PortControl: u8 {
        const INC =      bit!(7);
        const REMAP =    bits![3, 2]; // TODO
        const INC_RATE = bits![1, 0];
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

    borders:        Vec<(u16, bool)>
}

impl VRAM {
    pub fn new() -> Self {
        VRAM {
            data:           vec![0; VRAM_SIZE],

            port_control:   PortControl::default(),
            byte_addr:      0,

            borders:        Vec::new()
        }
    }

    pub fn set_port_control(&mut self, data: u8) {
        self.port_control = PortControl::from_bits_truncate(data);
        if self.port_control.contains(PortControl::REMAP) {
            panic!("Remap VRAM not implemented!");
        }
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

        self.set_dirty(self.byte_addr);

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }

    pub fn write_hi(&mut self, data: u8) {
        let addr = self.byte_addr.wrapping_add(1);
        self.data[addr as usize] = data;

        self.set_dirty(addr);

        if self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }

    // For use by renderer memory caches.
    pub fn ref_data<'a>(&'a self) -> &'a [u8] {
        &self.data
    }

    // Check if a region is dirty.
    pub fn dirty_range(&self, start_addr: u16) -> bool {
        if let Some((_, dirty)) = self.borders.iter().find(|(b, _)| *b == start_addr) {
            *dirty
        } else {
            false
        }
    }

    // Reset the dirty range once reading is finished.
    pub fn reset_dirty_range(&mut self) {
        for (_, dirty) in self.borders.iter_mut() {
            *dirty = false;
        }
    }

    // Set the borders of each region of VRAM.
    // Incoming vec must be ordered!
    pub fn set_borders(&mut self, borders: &[u16]) {
        self.borders = borders.iter().map(|b| (*b, true)).collect::<Vec<_>>();
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

    // Set a region to be dirty.
    #[inline]
    fn set_dirty(&mut self, addr: u16) {
        if let Some((_, dirty)) = self.borders.iter_mut().rev().find(|(a, _)| addr >= *a) {
            *dirty = true;
        }
    }
}