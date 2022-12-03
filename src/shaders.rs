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

pub mod light_obj_vert {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/lighting/light_obj.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

pub mod light_obj_frag {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/lighting/light_obj.frag",
    }
}

// TODO: find a better way to do this

pub struct ShaderModulePair {
    pub vert: Arc<ShaderModule>,
    pub frag: Arc<ShaderModule>,
}
pub struct Shaders {
    pub deferred: ShaderModulePair,
    pub directional: ShaderModulePair,
    pub ambient: ShaderModulePair,
    pub light_obj: ShaderModulePair
}
impl Shaders {
    pub fn default(device: &Arc<Device>) -> Self {
        Self { 
            deferred: ShaderModulePair { 
                vert: deferred_vert::load(device.clone()).unwrap(), 
                frag: deferred_frag::load(device.clone()).unwrap(),
            },
            directional: ShaderModulePair { 
                vert: directional_vert::load(device.clone()).unwrap(), 
                frag: directional_frag::load(device.clone()).unwrap(),
            },
            ambient: ShaderModulePair { 
                vert: ambient_vert::load(device.clone()).unwrap(), 
                frag: ambient_frag::load(device.clone()).unwrap(),
            },
            light_obj: ShaderModulePair { 
                vert: light_obj_vert::load(device.clone()).unwrap(), 
                frag: light_obj_frag::load(device.clone()).unwrap(),
            },
        }
    }
}