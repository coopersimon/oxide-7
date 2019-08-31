pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/video/vulkan/shaders/vertex.glsl"
    }
}

pub mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/video/vulkan/shaders/fragment.glsl"
    }
}