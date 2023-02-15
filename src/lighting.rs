use std::sync::Arc;

use nalgebra_glm::Vec3;
use vulkano::buffer::{CpuBufferPool, cpu_pool::CpuBufferPoolSubbuffer};

use crate::shaders::point_frag::ty::UPointLightData;

#[derive(Default, Debug, Clone)]
pub struct AmbientLight {
    pub color: [f32; 3],
    pub intensity: f32,
}
pub struct PointLight {
    position: Vec3,
    intensity: f32,
    color: [f32; 3],
    buffer: Option<Arc<CpuBufferPoolSubbuffer<UPointLightData>>>,
}


// TODO: changing light data at runtime might not work
impl PointLight {
    pub fn new(position: Vec3, intensity: f32, color: [f32; 3]) -> Self {
        Self {
            position,
            intensity,
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
    pub fn get_color(&self) -> &[f32; 3] {
        &self.color
    }
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }
    pub fn get_intensity(&self) -> f32 {
        self.intensity
    }
    pub(crate) fn get_buffer(
        &mut self,
        pool: &CpuBufferPool<UPointLightData>,
    ) -> Arc<CpuBufferPoolSubbuffer<UPointLightData>> {
        let position_arr = [self.position.x, self.position.y, self.position.z, 0.0];
        if let Some(buffer) = self.buffer.as_ref() {
            return buffer.clone();
        } else {
            let uniform_data = UPointLightData {
                position: position_arr.into(),
                intensity: self.intensity,
                color: self.color.into(),
            };
            let buffer = pool.from_data(uniform_data).unwrap();
            self.buffer = Some(buffer.clone());
            return buffer;
        }
    }
}