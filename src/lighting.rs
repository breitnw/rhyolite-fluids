use std::sync::Arc;

use vulkano::buffer::{CpuBufferPool, cpu_pool::CpuBufferPoolSubbuffer};

use crate::shaders::directional_frag;

#[derive(Default, Debug, Clone)]
pub struct AmbientLight {
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Default, Debug, Clone)]
pub struct DirectionalLight {
    pub position: [f32; 4],
    pub color: [f32; 3]
}

pub(crate) fn generate_directional_buffer(
    pool: &CpuBufferPool<directional_frag::ty::Directional_Light_Data>,
    light: &DirectionalLight,
) -> Arc<CpuBufferPoolSubbuffer<directional_frag::ty::Directional_Light_Data>> {
    let uniform_data = directional_frag::ty::Directional_Light_Data {
        position: light.position.into(),
        color: light.color.into(),
    };
    pool.from_data(uniform_data).unwrap()
}