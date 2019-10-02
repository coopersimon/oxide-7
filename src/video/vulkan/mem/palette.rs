// Palette memory.

use vulkano::{
    buffer::{
        CpuBufferPool,
        cpu_pool::CpuBufferPoolChunk
    },
    memory::pool::StdMemoryPool,
    device::Device,
    descriptor::descriptor_set::{
        FixedSizeDescriptorSet,
        PersistentDescriptorSetBuf,
    },
};

use std::sync::Arc;

use super::super::super::VideoMem;
use super::super::{
    RenderPipeline,
    uniforms::UniformCache
};

pub type PaletteBuffer = CpuBufferPoolChunk<u32, Arc<StdMemoryPool>>;

pub type PaletteDescriptorSet = Arc<FixedSizeDescriptorSet<
    Arc<RenderPipeline>,
    ( (), PersistentDescriptorSetBuf<PaletteBuffer> )
>>;

pub struct Palette {
    buffer_pool:        CpuBufferPool<u32>,
    current_bg_buffer:  Option<PaletteDescriptorSet>,
    current_obj_buffer: Option<PaletteDescriptorSet>
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
    pub fn create_bg_buffer(&mut self, mem: &mut VideoMem, uniform_cache: &mut UniformCache) {
        let cgram = mem.get_cgram();
        let buf = self.buffer_pool.chunk(
            cgram.chunks(4).map(|c| make32!(c[3], c[2], c[1], c[0]))
        ).unwrap();

        self.current_bg_buffer = Some(uniform_cache.bg_palette(buf));
    }

    pub fn create_obj_buffer(&mut self, mem: &mut VideoMem, uniform_cache: &mut UniformCache) {
        let cgram = mem.get_cgram();
        let buf = self.buffer_pool.chunk(
            cgram.chunks(4).skip(64).map(|c| make32!(c[3], c[2], c[1], c[0]))
        ).unwrap();

        self.current_obj_buffer = Some(uniform_cache.obj_palette(buf));
    }

    pub fn get_bg_palette_buffer(&self) -> PaletteDescriptorSet {
        self.current_bg_buffer.as_ref().unwrap().clone()
    }

    pub fn get_obj_palette_buffer(&self) -> PaletteDescriptorSet {
        self.current_obj_buffer.as_ref().unwrap().clone()
    }
}