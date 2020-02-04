// Palette memory.

use vulkano::{
    buffer::{
        ImmutableBuffer,
        BufferUsage
    },
    device::Queue,
    descriptor::descriptor_set::{
        FixedSizeDescriptorSet,
        PersistentDescriptorSetBuf,
    },
    sync::GpuFuture
};

use std::sync::Arc;

use super::super::super::VideoMem;
use super::super::{
    RenderPipeline,
    uniforms::UniformCache
};

pub type PaletteBuffer = Arc<ImmutableBuffer<[u32]>>;
pub type PaletteFuture = Box<dyn GpuFuture>;

pub type PaletteDescriptorSet = Arc<FixedSizeDescriptorSet<
    Arc<RenderPipeline>,
    ( (), PersistentDescriptorSetBuf<PaletteBuffer> )
>>;

pub struct Palette {
    queue:              Arc<Queue>,

    current_bg_buffer:  Option<PaletteDescriptorSet>,
    current_obj_buffer: Option<PaletteDescriptorSet>
}

impl Palette {
    pub fn new(queue: &Arc<Queue>) -> Self {
        Palette {
            queue:              queue.clone(),

            current_bg_buffer:  None,
            current_obj_buffer: None
        }
    }

    // Call if BG CGRAM is known to be dirty.
    pub fn clear_bg_buffer(&mut self) {
        self.current_bg_buffer = None;
    }

    // Call if OBJ CGRAM is known to be dirty.
    pub fn clear_obj_buffer(&mut self) {
        self.current_obj_buffer = None;
    }

    // Return cached palette buffer or create one if none is cached.
    pub fn get_bg_buffer(&mut self, mem: &VideoMem, uniform_cache: &mut UniformCache) -> (PaletteDescriptorSet, Option<PaletteFuture>) {
        if let Some(descriptor_set) = &self.current_bg_buffer {
            (descriptor_set.clone(), None)
        } else {
            let cgram = mem.get_cgram();
            let (buf, future) = ImmutableBuffer::from_iter(
                cgram.chunks(4).map(|c| make32!(c[3], c[2], c[1], c[0])),
                BufferUsage::uniform_buffer(),
                self.queue.clone()
            ).unwrap();

            let bg_buffer = uniform_cache.bg_palette(buf);

            self.current_bg_buffer = Some(bg_buffer.clone());
            (bg_buffer, Some(Box::new(future)))
        }
    }

    // Return cached palette buffer or create one if none is cached.
    pub fn get_obj_buffer(&mut self, mem: &VideoMem, uniform_cache: &mut UniformCache) -> (PaletteDescriptorSet, Option<PaletteFuture>) {
        if let Some(descriptor_set) = &self.current_obj_buffer {
            (descriptor_set.clone(), None)
        } else {
            let cgram = mem.get_cgram();
            let (buf, future) = ImmutableBuffer::from_iter(
                cgram.chunks(4).skip(64).map(|c| make32!(c[3], c[2], c[1], c[0])),
                BufferUsage::uniform_buffer(),
                self.queue.clone()
            ).unwrap();

            let obj_buffer = uniform_cache.obj_palette(buf);

            self.current_obj_buffer = Some(obj_buffer.clone());
            (obj_buffer, Some(Box::new(future)))
        }
    }
}