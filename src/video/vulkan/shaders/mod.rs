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

mod _refresh_files {
    #[allow(dead_code)]
    const S0: &str = include_str!("../shaders/bg/vertex.glsl");
    #[allow(dead_code)]
    const S1: &str = include_str!("../shaders/bg/fragment.glsl");
    #[allow(dead_code)]
    const S2: &str = include_str!("../shaders/obj/vertex.glsl");
    #[allow(dead_code)]
    const S3: &str = include_str!("../shaders/obj/fragment.glsl");
    #[allow(dead_code)]
    const S4: &str = include_str!("../shaders/debug/vertex.glsl");
    #[allow(dead_code)]
    const S5: &str = include_str!("../shaders/debug/fragment.glsl");
}