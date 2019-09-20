pub mod bg_vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/video/vulkan/shaders/bg/vertex.glsl"
    }
}

pub mod bg_fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/video/vulkan/shaders/bg/fragment.glsl"
    }
}

pub mod obj_vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/video/vulkan/shaders/obj/vertex.glsl"
    }
}

pub mod obj_fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/video/vulkan/shaders/obj/fragment.glsl"
    }
}

pub mod debug_vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/video/vulkan/shaders/debug/vertex.glsl"
    }
}

pub mod debug_fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/video/vulkan/shaders/debug/fragment.glsl"
    }
}