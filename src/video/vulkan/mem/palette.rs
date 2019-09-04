// Palette memory.

use vulkano::{
    buffer::CpuBufferPool,
    buffer::cpu_pool::CpuBufferPoolChunk,
    memory::pool::StdMemoryPool,
    device::Device
};

use std::sync::Arc;

pub type PaletteBuffer = CpuBufferPoolChunk<u16, Arc<StdMemoryPool>>;

#[derive(Clone)]
struct Palette {
    buffer_pool:    CpuBufferPool<u16>,
    current_buffer: Option<PaletteBuffer>
}

impl Palette {
    fn new(device: &Arc<Device>) -> Self {
        Palette {
            buffer_pool:    CpuBufferPool::uniform_buffer(device.clone()),
            current_buffer: None
        }
    }

    // TODO: should this return the buffer or the set?
    fn get_palette_buffer(&mut self) {
    }
}