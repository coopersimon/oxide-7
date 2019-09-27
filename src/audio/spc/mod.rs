// SPC-700 Audio processor
mod mem;

use bitflags::bitflags;

use mem::SPCBus;

bitflags! {
    #[derive(Default)]
    struct PSFlags: u8 {
        const N = bit!(7);  // Negative
        const V = bit!(6);  // Overflow
        const P = bit!(5);  // Direct page
        const B = bit!(4);  // Break
        const H = bit!(3);  // Half carry
        const I = bit!(2);  // Interrupt
        const Z = bit!(1);  // Zero
        const C = bit!(0);  // Carry
    }
}

const SPC_OP: usize = 1;   // Number of cycles for an internal op.

pub struct SPC {
    a:      u8,         // Accumulator
    x:      u8,         // X-Index
    y:      u8,         // Y-Index
    sp:     u8,         // Stack Pointer
    pc:     u16,        // Program Counter

    ps:     PSFlags,    // Program Status Word

    bus:    SPCBus
}

impl SPC {
    pub fn new() -> Self {
        SPC {
            a:      0,
            x:      0,
            y:      0,
            sp:     0,
            pc:     0xFFC0,

            ps:     PSFlags::default(),

            bus:    SPCBus::new()
        }
    }

    pub fn step(&mut self) {

    }
}

// Internal
impl SPC {
    fn execute_instruction(&mut self) {
        let instr = self.fetch();

        /*match instr {





            0x00 => self.nop(),
        }*/
    }

    fn clock_inc(&mut self, _cycles: usize) {

    }

    fn fetch(&mut self) -> u8 {
        let data = self.read_data(self.pc);
        self.pc = self.pc.wrapping_add(1);
        data
    }

    fn read_data(&mut self, addr: u16) -> u8 {
        let data = self.bus.read(addr);
        self.clock_inc(SPC_OP);
        data
    }

    fn write_data(&mut self, addr: u16, data: u8) {
        self.bus.write(addr, data);
        self.clock_inc(SPC_OP);
    }
}

// Instructions
impl SPC {
    fn nop(&mut self) {
        self.clock_inc(SPC_OP);
    }
}