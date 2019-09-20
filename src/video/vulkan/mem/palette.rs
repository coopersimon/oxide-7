// Palette memory.

use vulkano::{
    buffer::CpuBufferPool,
    buffer::cpu_pool::CpuBufferPoolChunk,
    memory::pool::StdMemoryPool,
    device::Device
};

use std::sync::Arc;

use super::super::super::VideoMem;

pub type PaletteBuffer = CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>;

pub struct Palette {
    buffer_pool:        CpuBufferPool<u32>,
    current_bg_buffer:  Option<PaletteBuffer>,
    current_obj_buffer: Option<PaletteBuffer>
}

impl Palette {
    pub fn new(device: &Arc<Device>) -> Self {
        Palette {
            buffer_pool:        CpuBufferPool::uniform_buffer(device.clone()),
            current_bg_buffer:  None,
            current_obj_buffer: None
        }
    }

    // Makes a new buffer and replaces the old one.
    pub fn create_bg_buffer(&mut self, mem: &mut VideoMem) {
        let cgram = mem.get_cgram();
        let buf = self.buffer_pool.chunk(
            cgram.chunks(4).map(|c| make32!(c[3], c[2], c[1], c[0]))
        ).unwrap();

        self.current_bg_buffer = Some(buf.clone());
    }

    pub fn create_obj_buffer(&mut self, mem: &mut VideoMem) {
        let cgram = mem.get_cgram();
        let buf = self.buffer_pool.chunk(
            cgram.chunks(4).skip(64).map(|c| make32!(c[3], c[2], c[1], c[0]))
        ).unwrap();

        self.current_obj_buffer = Some(buf.clone());
    }

    // TODO: should these return the buffer or the set?
    pub fn get_bg_palette_buffer(&self) -> PaletteBuffer {
        self.current_bg_buffer.as_ref().unwrap().clone()
    }

    pub fn get_obj_palette_buffer(&self) -> PaletteBuffer {
        self.current_obj_buffer.as_ref().unwrap().clone()
    }
}