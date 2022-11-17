// Deferred vertex shader
pub mod deferred_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/deferred.vs",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

// Deferred fragment shader
pub mod deferred_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/deferred.fs",
    }
}

// Lighting vertex shader
pub mod lighting_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/lighting.vs",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

// Lighting fragment shader
pub mod lighting_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/lighting.fs",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}