use std::sync::Arc;

use nalgebra_glm::Vec3;
use vulkano::buffer::{CpuBufferPool, cpu_pool::CpuBufferPoolSubbuffer};

use crate::shaders::directional_frag;

#[derive(Default, Debug, Clone)]
pub struct AmbientLight {
    pub color: [f32; 3],
    pub intensity: f32,
}
pub struct DirectionalLight {
    position: Vec3,
    color: [f32; 3],
    buffer: Option<Arc<CpuBufferPoolSubbuffer<directional_frag::ty::Directional_Light_Data>>>,
}

impl DirectionalLight {
    pub fn new(position: Vec3, color: [f32; 3]) -> Self {
        Self {
            position,
            color, 
            buffer: None,
        }
    }
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }
    pub fn get_position(&self) -> &Vec3 {
        &self.position
    }
    pub fn set_color(&mut self, color: [f32; 3]) {
        self.color = color;
    }
    pub(crate) fn get_buffer(
        &mut self,
        pool: &CpuBufferPool<directional_frag::ty::Directional_Light_Data>,
    ) -> Arc<CpuBufferPoolSubbuffer<directional_frag::ty::Directional_Light_Data>> {
        let position_arr = [self.position.x, self.position.y, self.position.z, 0.0];
        if let Some(buffer) = self.buffer.as_ref() {
            return buffer.clone();
        } else {
            let uniform_data = directional_frag::ty::Directional_Light_Data {
                position: position_arr.into(),
                color: self.color.into(),
            };
            let buffer = pool.from_data(uniform_data).unwrap();
            self.buffer = Some(buffer.clone());
            return buffer;
        }
    }
}