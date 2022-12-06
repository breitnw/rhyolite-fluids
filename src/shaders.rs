use std::sync::Arc;

use vulkano::{shader::ShaderModule, device::Device};

pub mod albedo_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/albedo.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod albedo_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/albedo.frag",
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

pub mod unlit_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/unlit.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod unlit_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/unlit.frag",
    }
}

// TODO: find a better way to do this

pub struct ShaderModulePair {
    pub vert: Arc<ShaderModule>,
    pub frag: Arc<ShaderModule>,
}
pub struct Shaders {
    pub albedo: ShaderModulePair,
    pub directional: ShaderModulePair,
    pub ambient: ShaderModulePair,
    pub unlit: ShaderModulePair
}
impl Shaders {
    pub fn default(device: &Arc<Device>) -> Self {
        Self { 
            albedo: ShaderModulePair { 
                vert: albedo_vert::load(device.clone()).unwrap(), 
                frag: albedo_frag::load(device.clone()).unwrap(),
            },
            directional: ShaderModulePair { 
                vert: directional_vert::load(device.clone()).unwrap(), 
                frag: directional_frag::load(device.clone()).unwrap(),
            },
            ambient: ShaderModulePair { 
                vert: ambient_vert::load(device.clone()).unwrap(), 
                frag: ambient_frag::load(device.clone()).unwrap(),
            },
            unlit: ShaderModulePair { 
                vert: unlit_vert::load(device.clone()).unwrap(), 
                frag: unlit_frag::load(device.clone()).unwrap(),
            },
        }
    }
}