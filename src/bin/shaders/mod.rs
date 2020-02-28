pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/bin/shaders/vertex.glsl"
    }
}

pub mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/bin/shaders/fragment.glsl"
    }
}

mod _refresh_files {
    #[allow(dead_code)]
    const VS: &str = include_str!("../shaders/vertex.glsl");
    #[allow(dead_code)]
    const FS: &str = include_str!("../shaders/fragment.glsl");
}