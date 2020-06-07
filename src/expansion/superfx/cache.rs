// Code cache for SuperFX.

#[derive(Clone, Copy)]
struct CacheLine {
    data: [u8; 16],
}

impl CacheLine {
    fn new() -> Self {
        Self {
            data:   [0; 16]
        }
    }

    fn is_loaded(&self) -> bool {
        self.data[15] != 0
    }

    fn try_read(&self, addr: u16) -> Option<u8> {
        if self.is_loaded() {
            Some(self.data[(addr & 0xF) as usize])
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.data[15] = 0;
    }
}

const CACHE_SIZE: u16 = 0x200;

pub enum CacheResult {
    InCache(u8),    // The requested data has been found inside the cache.
    Request,        // The requested data should be in the cache, but it currently is not.
    OutsideCache    // The requested data is outside the cache.
}

pub struct Cache {
    lines:  [CacheLine; 32],
    cbr:    u16,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            lines:  [CacheLine::new(); 32],
            cbr:    0,
        }
    }

    // Try read from cache.
    pub fn try_read(&self, addr: u16) -> CacheResult {
        let cache_addr = addr.wrapping_sub(self.cbr);
        if cache_addr < CACHE_SIZE {
            let cache_line = (cache_addr >> 4) as usize;
            match self.lines[cache_line].try_read(cache_addr) {
                Some(data) => CacheResult::InCache(data),
                None => CacheResult::Request
            }
        } else {
            CacheResult::OutsideCache
        }
    }

    // Fill the cache with the value specified.
    pub fn fill(&mut self, addr: u16, data: u8) {
        let cache_addr = addr.wrapping_sub(self.cbr);
        self.write(cache_addr, data);
    }

    // Read whatever happens to be at the cache location.
    // Addr should be between 0 and 0x1FF
    pub fn read(&self, addr: u16) -> u8 {
        self.lines[(addr >> 4) as usize].data[(addr & 0xF) as usize]
    }

    // Write to the location specified.
    // Addr should be between 0 and 0x1FF
    pub fn write(&mut self, addr: u16, data: u8) {
        self.lines[(addr >> 4) as usize].data[(addr & 0xF) as usize] = data;
    }

    pub fn set_cbr(&mut self, data: u16) {
        self.cbr = data;
        for line in &mut self.lines {
            line.clear();
        }
    }

    pub fn get_cbr(&self) -> u16 {
        self.cbr
    }
}