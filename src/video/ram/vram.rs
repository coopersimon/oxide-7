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
const INC_RATE_2: u8 = 0;
const INC_RATE_64: u8 = 1;
const INC_RATE_128: u8 = 2;
const INC_RATE_256: u8 = 3;

pub struct VRAM {
    data:           Vec<u8>,

    port_control:   PortControl,
    addr:           u16
}

impl VRAM {
    pub fn new() -> Self {
        VRAM {
            data:           vec![0; 64 * 1024],

            port_control:   PortControl::default(),
            addr:           0
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
        let ret = self.data[self.addr as usize];

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn read_hi(&mut self) -> u8 {
        let ret = self.data[self.addr.wrapping_add(1) as usize];

        if self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }

        ret
    }

    pub fn write_lo(&mut self, data: u8) {
        self.data[self.addr as usize] = data;

        if !self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }

    pub fn write_hi(&mut self, data: u8) {
        self.data[self.addr as usize] = data;

        if self.port_control.contains(PortControl::INC) {
            self.inc_addr();
        }
    }
}

// Internal
impl VRAM {
    #[inline]
    fn inc_addr(&mut self) {
        let inc_rate = match (self.port_control & PortControl::INC_RATE).bits() {
            INC_RATE_2   => 2,
            INC_RATE_64  => 64,
            INC_RATE_128 => 128,
            INC_RATE_256 => 256,
            _ => unreachable!()
        };

        self.addr = self.addr.wrapping_add(inc_rate);
    }
}