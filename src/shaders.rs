pub mod deferred_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/deferred.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod deferred_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/deferred.frag",
    }
}

pub mod directional_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/lighting/directional.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod directional_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/lighting/directional.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod ambient_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/lighting/ambient.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod ambient_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/lighting/ambient.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}