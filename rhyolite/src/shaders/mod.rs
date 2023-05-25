use std::sync::Arc;

use nalgebra_glm::Vec3;
use vulkano::{device::Device, shader::ShaderModule};

pub mod albedo_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/mesh/albedo.vert",
    }
}

pub mod albedo_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/mesh/albedo.frag",
    }
}

pub mod point_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/mesh/lighting/point.vert",
    }
}

pub mod point_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/mesh/lighting/point.frag",
    }
}

pub mod ambient_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/mesh/lighting/ambient.vert",
    }
}

pub mod ambient_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/mesh/lighting/ambient.frag",
    }
}

pub mod unlit_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/mesh/unlit.vert",
    }
}

pub mod unlit_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/mesh/unlit.frag",
    }
}

pub mod marched_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/marched/marched.vert",
    }
}

pub mod marched_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/marched/marched.frag",
    }
}

// TODO: find a better way to do this

pub struct ShaderModulePair {
    pub vert: Arc<ShaderModule>,
    pub frag: Arc<ShaderModule>,
}

impl ShaderModulePair {
    pub(crate) fn marched_default(device: &Arc<Device>) -> Self {
        Self {
            vert: marched_vert::load(device.clone()).unwrap(),
            frag: marched_frag::load(device.clone()).unwrap(),
        }
    }
}
pub struct Shaders {
    pub albedo: ShaderModulePair,
    pub point: ShaderModulePair,
    pub ambient: ShaderModulePair,
    pub unlit: ShaderModulePair,
}
impl Shaders {
    pub(crate) fn mesh_default(device: &Arc<Device>) -> Self {
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

/// A utility function to convert from a vec3 to a f32 array with length 4. Meant to aid with byte alignment for
/// descriptor sets.
pub(crate) fn expand_vec3(vec3: &Vec3) -> [f32; 4] {
    return [vec3.x, vec3.y, vec3.z, 0.0];
}
