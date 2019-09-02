// DMA Channel

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct DMAControl: u8 {
        const TRANSFER_DIR  = bit!(7);
        const HDMA_INDIRECT = bit!(6);
        const ADDR_DEC      = bit!(4);
        const FIXED_ADDR    = bit!(3);
        const TRANSFER_MODE = bit!(2) | bit!(1) | bit!(0);
    }
}

#[derive(Clone)]
pub struct DMAChannel {
    pub control:    DMAControl,
    pub b_bus_addr: u8,
    a_bus_addr:     u32,
    
    pub count:      u16,
    pub hdma_bank:  u8,

    pub hdma_table_addr:    u32,
    pub hdma_line_count:    u8,

    bytes_per_cycle:        u32,
}

impl DMAChannel {
    pub fn new() -> Self {
        DMAChannel {
            control:        DMAControl::default(),
            b_bus_addr:     0,
            a_bus_addr:     0,
            
            count:      0,
            hdma_bank:  0,

            hdma_table_addr:    0,
            hdma_line_count:    0,

            bytes_per_cycle:    0
        }
    }

    pub fn read(&self, addr: u8) -> u8 {
        match addr {
            0x0 => self.control.bits(),
            0x1 => self.b_bus_addr,
            0x2 => lo24!(self.a_bus_addr, u8),
            0x3 => mid24!(self.a_bus_addr),
            0x4 => hi24!(self.a_bus_addr),
            0x5 => lo!(self.count),
            0x6 => hi!(self.count),
            0x7 => self.hdma_bank,
            0x8 => lo24!(self.hdma_table_addr, u8),
            0x9 => mid24!(self.hdma_table_addr),
            0xA => self.hdma_line_count,
            _ => unreachable!()
        }
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        match addr {
            0x0 => {
                self.control = DMAControl::from_bits_truncate(data);
                self.bytes_per_cycle = match (self.control & DMAControl::TRANSFER_MODE).bits() {
                    0 => 1,
                    1 | 2 | 6 => 2,
                    3 | 4 | 5 | 7 => 4,
                    _ => unreachable!()
                };
            },
            0x1 => self.b_bus_addr = data,
            0x2 => self.a_bus_addr = set_lo24!(self.a_bus_addr, data),
            0x3 => self.a_bus_addr = set_mid24!(self.a_bus_addr, data),
            0x4 => self.a_bus_addr = set_hi24!(self.a_bus_addr, data),
            0x5 => self.count = set_lo!(self.count, data),
            0x6 => self.count = set_hi!(self.count, data),
            0x7 => self.hdma_bank = data,
            0x8 => self.hdma_table_addr = set_lo24!(self.hdma_table_addr, data),
            0x9 => self.hdma_table_addr = set_hi24!(self.hdma_table_addr, data),
            0xA => self.hdma_line_count = data,
            _ => unreachable!()
        }
    }

    pub fn get_a_bus_addr(&mut self) -> u32 {
        let ret = self.a_bus_addr;

        if !self.control.contains(DMAControl::FIXED_ADDR) {
            if !self.control.contains(DMAControl::ADDR_DEC) {
                self.a_bus_addr = self.a_bus_addr.wrapping_add(self.bytes_per_cycle);
            } else {
                self.a_bus_addr = self.a_bus_addr.wrapping_sub(self.bytes_per_cycle);
            }
        }

        ret
    }

    pub fn get_b_bus_addr(&mut self) -> u32 {
        (self.b_bus_addr as u32) | 0x2100
    }

    // Cycles for a single transfer block.
    pub fn get_cycles(&self) -> usize {
        match (self.control & DMAControl::TRANSFER_MODE).bits() {
            0 => 8,
            1 | 2 | 6 => 16,
            3 | 4 | 5 | 7 => 32,
            _ => unreachable!()
        }
    }

    // Decrement the bytes left and return true if the transfer is complete.
    pub fn decrement_count(&mut self) -> bool {
        self.count = self.count.wrapping_sub(self.bytes_per_cycle as u16);

        self.count == 0
    }

    // HDMA
    pub fn start_hdma(&mut self) {
        self.hdma_table_addr = self.a_bus_addr;
    }

    // Call this for every instruction in the table. Return false if the HDMA table is finished.
    pub fn init_instr(&mut self, line_count: u8) -> bool {
        self.hdma_line_count = line_count;

        if self.hdma_line_count == 0 {
            false
        } else {
            self.hdma_table_addr = self.hdma_table_addr.wrapping_add(1);
            true
        }
    }

    // Call this when the instruction is done, to set up the location of next instruction in the table.
    pub fn inc_table_addr(&mut self) {
        self.hdma_table_addr = self.hdma_table_addr.wrapping_add(self.bytes_per_cycle);
    }

    // Check if the instruction should just run once.
    pub fn once(&self) -> bool {
        !test_bit!(self.hdma_line_count, 7, u8)
    }

    pub fn get_line_count(&self) -> u8 {
        self.hdma_line_count & !bit!(7)
    }

    pub fn dec_line_count(&mut self) {
        self.hdma_line_count = self.get_line_count().wrapping_sub(1) | (self.hdma_line_count & bit!(7));
    }

    pub fn indirect_table_addr(&self, addr: u16) -> u32 {
        set_lo24!(self.a_bus_addr, addr, u16)
    }
}