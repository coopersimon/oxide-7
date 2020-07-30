// Cache for writing data back to RAM.

pub enum WritebackData {
    Byte(u8),
    Word(u8, u8),   // lo, hi
    None
}

pub struct WriteCache {
    data_lo:    Option<u8>,
    data_hi:    Option<u8>,
    pub bank:   u8,
    pub addr:   u16,
    cycles:     isize,

    writeback_duration:     isize,
}

impl WriteCache {
    pub fn new() -> Self {
        Self {
            data_lo:    None,
            data_hi:    None,
            bank:       0,
            addr:       0,
            cycles:     0,

            writeback_duration:     3,
        }
    }

    // Clock. Returns 0, 1 or 2 bytes to writeback to RAM.
    pub fn clock(&mut self, cycles: isize) -> WritebackData {
        if self.cycles > 0 {
            self.cycles -= cycles;

            if self.cycles <= 0 {
                let lo = self.data_lo.unwrap_or(0);

                if let Some(hi) = self.data_hi {
                    self.cycles += self.writeback_duration;
                    if self.cycles <= 0 {
                        self.cycles = 0;
                        return WritebackData::Word(lo, hi);
                    } else {
                        self.data_lo = self.data_hi;
                        self.data_hi = None;
                        self.addr = self.addr.wrapping_add(1);
                    }
                } else {
                    self.cycles = 0;
                }
                
                return WritebackData::Byte(lo);
            }
        }

        WritebackData::None
    }

    pub fn set_writeback_duration(&mut self, duration: isize) {
        self.writeback_duration = duration;
    }

    // Try and write a byte.
    // If a value != 0 is returned, there is a value in the cache, and the returned duration must be clocked to flush it.
    pub fn write_byte(&mut self, bank: u8, addr: u16, data: u8) -> isize {
        let flush_duration = self.flush_duration();
        if flush_duration > 0 {
            flush_duration
        } else {
            self.data_lo = Some(data);
            self.data_hi = None;
            self.bank = bank;
            self.addr = addr;
            self.cycles = self.writeback_duration;
            0
        }
    }

    // Try and write a byte.
    // If a value != 0 is returned, there is a value in the cache, and the returned duration must be clocked to flush it.
    pub fn write_word(&mut self, bank: u8, addr: u16, data: u16) -> isize {
        let flush_duration = self.flush_duration();
        if flush_duration > 0 {
            flush_duration
        } else {
            self.data_lo = Some(lo!(data));
            self.data_hi = Some(hi!(data));
            self.bank = bank;
            self.addr = addr;
            self.cycles = self.writeback_duration;
            0
        }
    }

    // Check how many cycles are left before flushing.
    pub fn flush_duration(&self) -> isize {
        if self.cycles > 0 {
            if let Some(_) = self.data_hi {
                self.cycles + self.writeback_duration
            } else {
                self.cycles
            }
        } else {
            0
        }
    }
}