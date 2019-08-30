// Vulkan renderer and data caches.

use winit::EventsLoop;

use super::VRamRef;

pub struct Renderer {
    mem: VRamRef
}

impl Renderer {
    pub fn new(mem: VRamRef, events_loop: &EventsLoop) -> Self {
        // Setup

        Renderer {
            mem: mem
        }
    }
}