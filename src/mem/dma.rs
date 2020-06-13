// DMA Channel

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct DMAControl: u8 {
        const TRANSFER_DIR  = bit!(7);
        const HDMA_INDIRECT = bit!(6);
        const ADDR_DEC      = bit!(4);
        const FIXED_ADDR    = bit!(3);
        const TRANSFER_MODE = bits![2, 1, 0];
    }
}

#[derive(Clone)]
pub struct DMAChannel {
    pub control:    DMAControl,
    pub b_bus_addr: u8,
    a_bus_addr:     u16,
    a_bus_bank:     u8,
    
    count:      u16,
    hdma_bank:  u8,

    hdma_table_addr:    u16,
    hdma_line_count:    u8,
    hdma_repeat:        bool,

    bytes_per_cycle:    u16,
}

impl DMAChannel {
    pub fn new() -> Self {
        DMAChannel {
            control:        DMAControl::default(),
            b_bus_addr:     0,
            a_bus_addr:     0,
            a_bus_bank:     0,
            
            count:      0,
            hdma_bank:  0,

            hdma_table_addr:    0xFF,
            hdma_line_count:    0,
            hdma_repeat:        false,

            bytes_per_cycle:    0
        }
    }

    pub fn read(&self, addr: u8) -> u8 {
        match addr {
            0x0 => self.control.bits(),
            0x1 => self.b_bus_addr,
            0x2 => lo!(self.a_bus_addr),
            0x3 => hi!(self.a_bus_addr),
            0x4 => self.a_bus_bank,
            0x5 => lo!(self.count),
            0x6 => hi!(self.count),
            0x7 => self.hdma_bank,
            0x8 => lo!(self.hdma_table_addr),
            0x9 => hi!(self.hdma_table_addr),
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
            0x2 => self.a_bus_addr = set_lo!(self.a_bus_addr, data),
            0x3 => self.a_bus_addr = set_hi!(self.a_bus_addr, data),
            0x4 => self.a_bus_bank = data,
            0x5 => self.count = set_lo!(self.count, data),
            0x6 => self.count = set_hi!(self.count, data),
            0x7 => self.hdma_bank = data,
            0x8 => self.hdma_table_addr = set_lo!(self.hdma_table_addr, data),
            0x9 => self.hdma_table_addr = set_hi!(self.hdma_table_addr, data),
            0xA => self.hdma_line_count = data,
            _ => unreachable!()
        }
    }

    pub fn get_a_bus_addr(&mut self) -> u32 {
        let ret = make24!(self.a_bus_bank, self.a_bus_addr);

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
        make24!(0, 0x21, self.b_bus_addr)
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
        self.count = self.count.wrapping_sub(1);

        self.count == 0
    }

    // HDMA
    pub fn start_hdma(&mut self) {
        self.hdma_table_addr = self.a_bus_addr;
        self.hdma_line_count = 0;
    }

    // Call this for every instruction in the table. Return false if the HDMA table is finished.
    pub fn hdma_init_instr(&mut self, line_count: u8) -> bool {
        if line_count == 0 {
            false
        } else {
            let repeat_line_count = line_count.wrapping_sub(1);
            self.hdma_line_count = repeat_line_count & 0x7F;
            self.hdma_repeat = test_bit!(repeat_line_count, 7, u8);
            self.hdma_table_addr = self.hdma_table_addr.wrapping_add(1);
            true
        }
    }

    // Call this every line. Returns false when ready for next instruction.
    pub fn hdma_step_line(&mut self) -> bool {
        if self.hdma_line_count == 0 {
            false
        } else {
            self.hdma_line_count = self.hdma_line_count.wrapping_sub(1) & 0x7F;
            true
        }
    }

    pub fn get_hdma_table_addr(&self) -> u32 {
        make24!(self.a_bus_bank, self.hdma_table_addr)
    }

    // Check if the instruction should repeat or run once and skip.
    pub fn should_repeat(&self) -> bool {
        self.hdma_repeat
    }

    // Set address for indirect data, and inc table address.
    pub fn set_indirect_table_addr(&mut self, addr: u16) {
        self.count = addr;
        self.hdma_table_addr = self.hdma_table_addr.wrapping_add(2);   // Addresses are 2 bytes.
    }

    // Get address of data to use, and increment.
    pub fn get_data_addr(&mut self) -> u32 {
        if self.control.contains(DMAControl::HDMA_INDIRECT) {
            let addr = make24!(self.hdma_bank, self.count);
            self.count = self.count.wrapping_add(self.bytes_per_cycle);
            addr
        } else {
            let addr = self.get_hdma_table_addr();
            self.hdma_table_addr = self.hdma_table_addr.wrapping_add(self.bytes_per_cycle);
            addr
        }
    }
}

// Debug
impl DMAChannel {
    #[allow(dead_code)]
    pub fn print_dma(&self) {
        println!("CTRL: {:b}, B_ADDR: {:X}, A_ADDR: {:X}_{:X}, count: {:X}", self.control.bits(), self.b_bus_addr, self.a_bus_bank, self.a_bus_addr, self.count);
    }

    #[allow(dead_code)]
    pub fn print_hdma(&self) {
        println!("CTRL: {:b}, B_ADDR: {:X}, A_ADDR: {:X}, bank: {:X}, line count: {:X}, table addr: {:X}, indirect addr: {:X}", self.control.bits(), self.b_bus_addr, self.a_bus_addr, self.hdma_bank, self.hdma_line_count, self.hdma_table_addr, self.count);
    }
}