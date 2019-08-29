// PPU

mod ram;

use winit::EventsLoop;

use ram::VideoMem;

pub struct PPU {
    mem: VideoMem
    // renderer
}

impl PPU {
    pub fn new(/*events_loop: EventsLoop*/) -> Self {
        PPU {
            mem: VideoMem::new()
        }
    }

    pub fn read_mem(&mut self, addr: u8) -> u8 {
        self.mem.read(addr)
    }

    pub fn write_mem(&mut self, addr: u8, data: u8) {
        self.mem.write(addr, data);
    }
}