// Dealing with uniforms and their descriptors.

use vulkano::{
    device::Device,
    sampler::{
        Filter,
        MipmapMode,
        Sampler,
        SamplerAddressMode
    },
    descriptor::descriptor_set::FixedSizeDescriptorSetsPool,
};

use std::sync::Arc;

use super::{
    mem::{
        palette::{
            PaletteBuffer,
            PaletteDescriptorSet
        },
        texturecache::{
            BGImageDescriptorSet,
            ObjImageDescriptorSet,
            PatternImage
        }
    },
    RenderPipeline
};

pub struct UniformCache {
    sampler:        Arc<Sampler>,
    bg_set_pools:   Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
    obj_set_pools:  Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
}

impl UniformCache {
    pub fn new(device: &Arc<Device>, bg_pipeline: &Arc<RenderPipeline>, obj_pipeline: &Arc<RenderPipeline>) -> Self {
        // Make the sampler for the textures.
        let sampler = Sampler::new(
            device.clone(),
            Filter::Nearest,
            Filter::Nearest,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0, 1.0, 0.0, 0.0
        ).expect("Couldn't create sampler!");

        // Make descriptor set pools.
        let bg_set_pools = vec![
            FixedSizeDescriptorSetsPool::new(bg_pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(bg_pipeline.clone(), 1)
        ];

        let obj_set_pools = vec![
            FixedSizeDescriptorSetsPool::new(obj_pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(obj_pipeline.clone(), 1)
        ];

        UniformCache {
            sampler:        sampler,
            bg_set_pools:   bg_set_pools,
            obj_set_pools:  obj_set_pools,
        }
    }

    // Get a descriptor set for a sampled image, for the background tiles.
    pub fn bg_image(&mut self, image: PatternImage) -> BGImageDescriptorSet {
        Arc::new(self.bg_set_pools[0].next()
            .add_sampled_image(image, self.sampler.clone()).unwrap()
            .build().unwrap())
    }

    // Get a descriptor set for a sampled images, for the sprites.
    pub fn obj_images(&mut self, image_0: PatternImage, image_n: PatternImage) -> ObjImageDescriptorSet {
        Arc::new(self.obj_set_pools[0].next()
            .add_sampled_image(image_0, self.sampler.clone()).unwrap()
            .add_sampled_image(image_n, self.sampler.clone()).unwrap()
            .build().unwrap())
    }

    // Get a descriptor set for a palette, for the background.
    pub fn bg_palette(&mut self, palette_buffer: PaletteBuffer) -> PaletteDescriptorSet {
        Arc::new(self.bg_set_pools[1].next()
            .add_buffer(palette_buffer).unwrap()
            .build().unwrap())
    }

    // Get a descriptor set for a palette, for the sprites.
    pub fn obj_palette(&mut self, palette_buffer: PaletteBuffer) -> PaletteDescriptorSet {
        Arc::new(self.obj_set_pools[1].next()
            .add_buffer(palette_buffer).unwrap()
            .build().unwrap())
    }

    // TODO: "push constants".
}