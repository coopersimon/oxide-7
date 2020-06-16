// Loading from ROM

use super::super::constants::ROM_WAIT_CYCLES;

pub struct ROMCache {
    bank:   u8,
    addr:   u16,
    data:   u8,
    cycles: usize,
}

impl ROMCache {
    pub fn new() -> Self {
        Self {
            bank:   0,
            addr:   0,
            data:   0,
            cycles: 0,
        }
    }

    // Returns true if data is ready to be loaded.
    pub fn clock(&mut self) -> bool {
        if self.cycles > 0 {
            self.cycles -= 1;
            self.cycles == 0
        } else {
            false
        }
    }

    pub fn start_loading(&mut self, bank: u8, addr: u16) {
        // TODO: check - are we ever waiting on more than 1 ROM byte at a time?
        self.bank = bank;
        self.addr = addr;
        self.cycles = ROM_WAIT_CYCLES - 1;
    }

    // Called after clock returns true.
    pub fn load_data(&mut self, data: u8) {
        //println!("Got {:X}", data);
        self.data = data;
    }

    pub fn try_read(&mut self, bank: u8, addr: u16) -> Option<u8> {
        //println!("Try read {:X}_{:X}", bank, addr);
        if self.cycles == 0 {   // We are done with the previous load.
            if self.bank == bank && self.addr == addr {
                //println!("Have {:X}", self.data);
                Some(self.data)
            } else {
                //println!("Start loading");
                self.start_loading(bank, addr);
                None
            }
        } else {
            None
        }
    }

    pub fn get_bank(&self) -> u8 {
        self.bank
    }

    pub fn get_addr(&self) -> u16 {
        self.addr
    }
}