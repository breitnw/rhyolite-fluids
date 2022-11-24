use std::sync::Arc;

use vulkano::{shader::ShaderModule, device::Device};

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

pub struct Shaders {
    pub deferred_vert: Arc<ShaderModule>,
    pub deferred_frag: Arc<ShaderModule>,
    pub directional_vert: Arc<ShaderModule>,
    pub directional_frag: Arc<ShaderModule>,
    pub ambient_vert: Arc<ShaderModule>,
    pub ambient_frag: Arc<ShaderModule>,
}
impl Shaders {
    pub fn default(device: &Arc<Device>) -> Self {
        Self { 
            deferred_vert: deferred_vert::load(device.clone()).unwrap(),
            deferred_frag: deferred_frag::load(device.clone()).unwrap(),
            directional_vert: directional_vert::load(device.clone()).unwrap(),
            directional_frag: directional_frag::load(device.clone()).unwrap(),
            ambient_vert: ambient_vert::load(device.clone()).unwrap(),
            ambient_frag: ambient_frag::load(device.clone()).unwrap(),
        }
    }
}