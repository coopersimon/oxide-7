// VRAM: background maps and pattern data.

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    struct PortControl: u8 {
        const INC =      bit!(7);
        const REMAP =    bits![3, 2];
        const INC_RATE = bits![1, 0];
    }
}

// Address increment rates.
const INC_RATE_1: u8 = 0;
const INC_RATE_32: u8 = 1;

// Size of VRAM (64kB, or 2^16)
const VRAM_SIZE: usize = 64 * 1024;

pub struct VRAM {
    data:           Vec<u8>,

    port_control:   PortControl,
    byte_addr:      u16,

    pattern_regions:    Vec<((u16, u16), bool)>
}

impl VRAM {
    pub fn new() -> Self {
        VRAM {
            data:           vec![0; VRAM_SIZE],

            port_control:   PortControl::default(),
            byte_addr:      0,

            pattern_regions:    Vec::new()
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
        let ret = self.data[self.remap_addr() as usize];

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn read_hi(&mut self) -> u8 {
        let ret = self.data[self.remap_addr().wrapping_add(1) as usize];

        if self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn write_lo(&mut self, data: u8) {
        let addr = self.remap_addr();
        self.data[addr as usize] = data;

        self.set_dirty(addr);

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }

    pub fn write_hi(&mut self, data: u8) {
        let addr = self.remap_addr().wrapping_add(1);
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
        if let Some((_, dirty)) = self.pattern_regions.iter().find(|((b, _), _)| *b == start_addr) {
            *dirty
        } else {
            false
        }
    }

    // Reset the dirty range once reading is finished.
    pub fn reset_dirty_range(&mut self, read: &[u16]) {
        for ((start, _), dirty) in self.pattern_regions.iter_mut() {
            if read.contains(start) {
                *dirty = false;
            }
        }
    }

    // Set the borders of each region of VRAM pattern memory.
    pub fn set_pattern_regions(&mut self, regions: Vec<(u16, u16)>) {
        self.pattern_regions = regions.iter().cloned().map(|r| (r, true)).collect::<Vec<_>>();
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
        for ((start, end), dirty) in self.pattern_regions.iter_mut() {
            if (addr >= *start) && (addr <= *end) {
                *dirty = true;
            }
        }
    }

    #[inline]
    fn remap_addr(&self) -> u16 {
        match (self.port_control & PortControl::REMAP).bits() >> 2 {
            0 => self.byte_addr,
            1 => {
                let upper = self.byte_addr & 0xFE00;
                let middle = (self.byte_addr & 0x01C0) >> 6;
                let lower = (self.byte_addr & 0x003F) << 3;
                upper | lower | middle
            },
            2 => {
                let upper = self.byte_addr & 0xFC00;
                let middle = (self.byte_addr & 0x0380) >> 7;
                let lower = (self.byte_addr & 0x007F) << 3;
                upper | lower | middle
            },
            3 => {
                let upper = self.byte_addr & 0xF800;
                let middle = (self.byte_addr & 0x0700) >> 8;
                let lower = (self.byte_addr & 0x00FF) << 3;
                upper | lower | middle
            },
            _ => unreachable!()
        }
    }
}