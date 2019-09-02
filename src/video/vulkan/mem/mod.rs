// Converting native VRAM, CGRAM and OAM into Vulkan structures.

pub mod patternmem;

use crate::video::VRamRef;
use patternmem::*;

pub struct MemoryCache {
    native_mem:     VRamRef,

    pattern_mem:    Vec<PatternMem>
}

impl MemoryCache {
    pub fn new(vram: VRamRef) -> Self {
        MemoryCache {
            native_mem:     vram,

            pattern_mem:    Vec::new()
        }
    }

    // Retrieve structures.
    /*pub fn get_bg_1_image(&mut self) -> (PatternImage, PatternFuture) {
        
    }*/


}