// Reading and writing with RAM.

use super::super::constants::RAM_WAIT_CYCLES;

pub enum Mode {
    ReadByte,   // Reading from RAM
    ReadWord,
    WriteByte(u8),  // Writing to RAM
    WriteWord(u16),
    PixelCacheWrite(Vec<u8>), // Writing contents of pixel cache to RAM.
    PixelCacheRead(usize),    // Reading RAM into pixel cache.
}

pub struct RAMCache {
    bank:       u8,
    addr:       u16,
    mode:       Mode,

    data_lo:    u8,
    data_hi:    u8,
    cycles:     usize,
}

impl RAMCache {
    pub fn new() -> Self {
        Self {
            bank:       0,
            addr:       0,
            mode:       Mode::ReadByte,
            data_lo:    0,
            data_hi:    0,
            cycles:     0,
        }
    }

    // Returns true if data is ready to be loaded.
    pub fn clock(&mut self) -> Option<Mode> {
        if self.cycles > 0 {
            self.cycles -= 1;
            if self.cycles == 0 {
                return Some(std::mem::replace(&mut self.mode, Mode::ReadByte));
            }
        }

        None
    }

    // Try and start the operation mode. If true is returned, it started OK and wait begins.
    // If false is returned, the existing data needs to be waited upon.
    pub fn try_start_operation(&mut self, bank: u8, addr: u16, mode: Mode) -> bool {
        if self.cycles != 0 {
            false
        } else {
            self.bank = bank;
            self.addr = addr;
            self.cycles = match &mode {
                Mode::PixelCacheWrite(d) => RAM_WAIT_CYCLES * d.len(),
                Mode::PixelCacheRead(n) => RAM_WAIT_CYCLES * n,
                Mode::ReadWord | Mode::WriteWord(_) => RAM_WAIT_CYCLES * 2,
                _ => RAM_WAIT_CYCLES
            } - 1;
            self.mode = mode;
            true
        }
    }

    // Called after clock returns true.
    pub fn load_data_lo(&mut self, data: u8) {
        self.data_lo = data;
    }

    // Called after clock returns true.
    pub fn load_data_hi(&mut self, data: u8) {
        self.data_hi = data;
    }

    pub fn try_read_byte(&mut self, bank: u8, addr: u16) -> Option<u8> {
        if self.cycles == 0 {
            if self.bank == bank && self.addr == addr {
                Some(self.data_lo)
            } else {
                self.try_start_operation(bank, addr, Mode::ReadByte);
                None
            }
        } else {
            None
        }
    }

    pub fn try_read_word(&mut self, bank: u8, addr: u16) -> Option<u16> {
        if self.cycles == 0 {
            if self.bank == bank && self.addr == addr {
                Some(make16!(self.data_hi, self.data_lo))
            } else {
                self.try_start_operation(bank, addr, Mode::ReadWord);
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

    pub fn is_idle(&self) -> bool {
        self.cycles == 0
    }
}