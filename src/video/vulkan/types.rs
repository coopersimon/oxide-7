// Types used in renderer.

use vulkano::{
    framebuffer::RenderPassAbstract,
    pipeline::{
        GraphicsPipeline,
        vertex::SingleBufferDefinition
    },
    descriptor::pipeline_layout::PipelineLayoutAbstract,
    memory::pool::StdMemoryPool,
    buffer::cpu_pool::CpuBufferPoolChunk,
};

use std::sync::Arc;

pub type RenderPipeline = GraphicsPipeline<
    SingleBufferDefinition<Vertex>,
    Box<dyn PipelineLayoutAbstract + Send + Sync>,
    Arc<dyn RenderPassAbstract + Send + Sync>
>;

// TODO: move this and other data elsewhere
#[derive(Clone, Copy)]
pub enum TexSide {
    Left =  0 << 16,
    Right = 1 << 16
}

#[derive(Clone, Copy)]
pub enum VertexSide {
    Left =  0 << 21,
    Right = 1 << 21
}

// Individual Vertex.
#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

vulkano::impl_vertex!(Vertex, position, data);

// Push constants used in BG shaders.
#[derive(Copy, Clone, Debug)]
pub struct BGPushConstants {
    pub tex_size:       [f32; 2],   // Size of individual tile texture in map.
    pub atlas_size:     [f32; 2],   // Size of texture atlas in tiles.
    pub tile_size:      [f32; 2],   // Width of tile relative to the viewport, height of line relative to the viewport.
    pub map_size:       [f32; 2],   // Size of tile map relative to the viewport.
    pub vertex_offset:  [f32; 2],   // Offset to apply to vertices for scrolling.   // TODO: use a different shader for sprites.
    pub palette_offset: u32,        // Offset for palette used by BG (in colours).
    pub palette_size:   u32,        // Size of palettes used.
    pub priority:       u32,        // Either 0 or 1<<13
    pub tex_pixel_height: f32       // Height of individual tile in pixels.
}

impl BGPushConstants {
    pub fn set_priority(mut self) -> BGPushConstants {
        self.priority = 1 << 13;
        self
    }
}

// Push constants used in Sprite shaders.
#[derive(Copy, Clone, Debug)]
pub struct ObjPushConstants {
    pub small_tex_size: [f32; 2],
    pub large_tex_size: [f32; 2],
}

pub type VertexBuffer = CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>;
