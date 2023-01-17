use std::sync::Arc;

use vulkano::{shader::ShaderModule, device::Device};

pub mod albedo_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/mesh/albedo.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod albedo_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/mesh/albedo.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod point_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/mesh/lighting/point.vert",
    }
}

pub mod point_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/mesh/lighting/point.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
} 

pub mod ambient_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/mesh/lighting/ambient.vert",
    }
}

pub mod ambient_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/mesh/lighting/ambient.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod unlit_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/mesh/unlit.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod unlit_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/mesh/unlit.frag",
    }
}

pub mod marched_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/marched/marched.vert",
    }
}

pub mod marched_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/marched/marched.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

// TODO: find a better way to do this

pub struct ShaderModulePair {
    pub vert: Arc<ShaderModule>,
    pub frag: Arc<ShaderModule>,
}
pub struct Shaders {
    pub albedo: ShaderModulePair,
    pub point: ShaderModulePair,
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
            point: ShaderModulePair { 
                vert: point_vert::load(device.clone()).unwrap(), 
                frag: point_frag::load(device.clone()).unwrap(),
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