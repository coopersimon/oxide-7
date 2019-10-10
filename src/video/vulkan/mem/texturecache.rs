// Vulakn-specific texture stuff. Using a sampled texture atlas to map tile data onto.

use vulkano::{
    device::{
        Device, Queue
    },
    descriptor::descriptor_set::{
        FixedSizeDescriptorSet,
        PersistentDescriptorSetImg,
        PersistentDescriptorSetSampler
    },
    image::{
        Dimensions,
        immutable::ImmutableImage
    },
    format::{
        R8Uint
    },
    sync::GpuFuture
};

use std::sync::Arc;

use crate::video::{
    VideoMem,
    vulkan::{
        RenderPipeline,
        uniforms::UniformCache
    },
    patternmem::*
};

pub type PatternImage = Arc<ImmutableImage<R8Uint>>;
pub type PatternFuture = Box<dyn GpuFuture>;

pub type BGImageDescriptorSet = Arc<FixedSizeDescriptorSet<
    Arc<RenderPipeline>,
    (( (), PersistentDescriptorSetImg<PatternImage> ), PersistentDescriptorSetSampler)
>>;

pub type ObjImageDescriptorSet = Arc<FixedSizeDescriptorSet<
    Arc<RenderPipeline>,
    (((( (), PersistentDescriptorSetImg<PatternImage> ),
        PersistentDescriptorSetSampler ),
        PersistentDescriptorSetImg<PatternImage> ),
        PersistentDescriptorSetSampler
    )
>>;

// Texture cache for background patterns.
pub struct BGTexCache {
    pattern_mem:    PatternMem,
    width:          u32,    // In pixels
    height:         u32,    // In pixels

    // Vulkan
    queue:          Arc<Queue>,
    descriptor:     Option<BGImageDescriptorSet>,
}

impl BGTexCache {
    pub fn new(queue: &Arc<Queue>, width: u32, height: u32, bits_per_pixel: BitsPerPixel) -> Self {
        BGTexCache {
            pattern_mem:    PatternMem::new(bits_per_pixel),
            width:          width,
            height:         height,

            queue:          queue.clone(),
            descriptor:     None,
        }
    }

    // Call if VRAM is known to be dirty.
    pub fn clear_image(&mut self, mem: &VideoMem) {
        if mem.vram_is_dirty(self.pattern_mem.get_start_addr()) {
            self.descriptor = None;
        }
    }

    // Set the start address of the data.
    pub fn set_addr(&mut self, start_addr: u16, height: u32) {
        self.height = height;

        self.pattern_mem.set_addr(start_addr, self.width as usize, self.height as usize);
    }

    // Return the BPP.
    pub fn get_bits_per_pixel(&self) -> BitsPerPixel {
        self.pattern_mem.get_bits_per_pixel()
    }

    // Return the start address of the data.
    pub fn get_start_addr(&self) -> u16 {
        self.pattern_mem.get_start_addr()
    }

    // Get the size of the tex in pixels.
    pub fn get_size(&self) -> (f32, f32) {
        (self.width as f32, self.height as f32)
    }

    // Return cached image or create one if none is cached.
    pub fn get_image(&mut self, mem: &VideoMem, uniform_cache: &mut UniformCache) -> (BGImageDescriptorSet, Option<PatternFuture>) {
        if let Some(descriptor_set) = &self.descriptor {
            (descriptor_set.clone(), None)
        } else {
            let data = &mem.get_vram()[(self.pattern_mem.get_start_addr() as usize)..(self.pattern_mem.get_end_addr() as usize)];
            let texture_data = self.pattern_mem.make_image(data, self.width as usize, self.height as usize);

            //println!("BG img");
            let (image, future) = ImmutableImage::from_iter(
                texture_data.drain(..),
                Dimensions::Dim2d { width: self.width, height: self.height },
                R8Uint,
                self.queue.clone()
            ).expect("Couldn't create image.");

            let descriptor_set = uniform_cache.bg_image(image);

            self.descriptor = Some(descriptor_set.clone());
            (descriptor_set, Some(Box::new(future)))
        }
    }
}

const OBJ_PATTERN_SIZE: usize = 16 * 8;   // Object pattern mem width and height in pixels.

// Texture cache for sprites.
pub struct ObjTexCache {
    pattern_mem_0:  PatternMem,
    pattern_mem_n:  PatternMem,

    // Vulkan
    device:         Arc<Device>,
    queue:          Arc<Queue>,
    descriptor:     Option<ObjImageDescriptorSet>,

    image_0:        Option<PatternImage>,
    image_n:        Option<PatternImage>
}

impl ObjTexCache {
    pub fn new(device: &Arc<Device>, queue: &Arc<Queue>) -> Self {
        ObjTexCache {
            pattern_mem_0:  PatternMem::new(BitsPerPixel::_4),
            pattern_mem_n:  PatternMem::new(BitsPerPixel::_4),

            device:         device.clone(),
            queue:          queue.clone(),
            descriptor:     None,

            image_0:        None,
            image_n:        None
        }
    }

    // Call if VRAM is known to be dirty.
    pub fn clear_images(&mut self, mem: &VideoMem) {
        if mem.vram_is_dirty(self.pattern_mem_0.get_start_addr()) {
            self.descriptor = None;
            self.image_0 = None;
        }
        if mem.vram_is_dirty(self.pattern_mem_n.get_start_addr()) {
            self.descriptor = None;
            self.image_n = None;
        }
    }

    // Set the start address of the data.
    // TODO: this can be done in a single method
    pub fn set_addr_0(&mut self, start_addr: u16) {
        self.pattern_mem_0.set_addr(start_addr, OBJ_PATTERN_SIZE, OBJ_PATTERN_SIZE);
    }

    pub fn set_addr_n(&mut self, start_addr: u16) {
        self.pattern_mem_n.set_addr(start_addr, OBJ_PATTERN_SIZE, OBJ_PATTERN_SIZE);
    }

    // Return the start address of the data.
    pub fn get_start_addr_0(&self) -> u16 {
        self.pattern_mem_0.get_start_addr()
    }

    pub fn get_start_addr_n(&self) -> u16 {
        self.pattern_mem_n.get_start_addr()
    }

    // Return cached images or create if none is cached.
    pub fn get_images(&mut self, mem: &VideoMem, uniform_cache: &mut UniformCache) -> (ObjImageDescriptorSet, Option<PatternFuture>) {
        if let Some(descriptor_set) = &self.descriptor {
            (descriptor_set.clone(), None)
        } else {
            let (image_0, future_0) = if let Some(image) = &self.image_0 {
                (image.clone(), Box::new(vulkano::sync::now(self.device.clone())) as Box<dyn GpuFuture>)
            } else {
                let data = &mem.get_vram()[(self.pattern_mem_0.get_start_addr() as usize)..(self.pattern_mem_0.get_end_addr() as usize)];
                let texture_data = self.pattern_mem_0.make_image(data, OBJ_PATTERN_SIZE, OBJ_PATTERN_SIZE);

                //println!("Obj img 0");
                let (image, future) = ImmutableImage::from_iter(
                    texture_data.drain(..),
                    Dimensions::Dim2d { width: OBJ_PATTERN_SIZE as u32, height: OBJ_PATTERN_SIZE as u32 },
                    R8Uint,
                    self.queue.clone()
                ).expect("Couldn't create image.");
                (image, Box::new(future) as Box<_>)
            };

            let (image_n, future_n) = if let Some(image) = &self.image_n {
                (image.clone(), Box::new(vulkano::sync::now(self.device.clone())) as Box<dyn GpuFuture>)
            } else {
                let data = &mem.get_vram()[(self.pattern_mem_n.get_start_addr() as usize)..(self.pattern_mem_n.get_end_addr() as usize)];
                let texture_data = self.pattern_mem_n.make_image(data, OBJ_PATTERN_SIZE, OBJ_PATTERN_SIZE);

                //println!("Obj img n");
                let (image, future) = ImmutableImage::from_iter(
                    texture_data.drain(..),
                    Dimensions::Dim2d { width: OBJ_PATTERN_SIZE as u32, height: OBJ_PATTERN_SIZE as u32 },
                    R8Uint,
                    self.queue.clone()
                ).expect("Couldn't create image.");
                (image, Box::new(future) as Box<_>)
            };

            let descriptor_set = uniform_cache.obj_images(image_0, image_n);

            self.descriptor = Some(descriptor_set.clone());
            (descriptor_set, Some(Box::new(future_0.join(future_n)) as PatternFuture))
        }
    }
}