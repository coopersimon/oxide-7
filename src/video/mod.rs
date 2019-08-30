// PPU
// Owns the video memory and is responsible for communicating with the renderer.

mod ram;

mod vulkan;

use std::{
    sync::{Arc, Mutex},
    thread
};

use winit::EventsLoop;

use ram::VideoMem;

type VRamRef = Arc<Mutex<VideoMem>>;

pub struct PPU {
    mem: VRamRef
}

impl PPU {
    pub fn new(events_loop: &EventsLoop) -> Self {
        let mem = Arc::new(Mutex::new(VideoMem::new()));
        let renderer = vulkan::Renderer::new(mem.clone(), events_loop);
        // Start video thread.
        thread::spawn(move || {
            let r = renderer;
            // Start video thread...
        });

        PPU {
            mem: mem
        }
    }

    // fn frame...?

    pub fn read_mem(&mut self, addr: u8) -> u8 {
        self.mem.lock().unwrap().read(addr)
    }

    pub fn write_mem(&mut self, addr: u8, data: u8) {
        self.mem.lock().unwrap().write(addr, data);
    }
}