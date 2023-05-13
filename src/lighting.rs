use nalgebra_glm::Vec3;
use vulkano::buffer::Subbuffer;

use crate::renderer::staging::{IntoPersistentUniform, UniformSrc};
use crate::shaders::{ambient_frag, expand_vec3, marched_frag, point_frag};

// TODO: ideally make the get_buffer thing a trait

#[derive(Default, Clone)]
pub struct AmbientLight {
    color: Vec3,
    intensity: f32,
    subbuffer: Option<Subbuffer<ambient_frag::UAmbientLightData>>,
}

impl AmbientLight {
    pub fn new(color: Vec3, intensity: f32) -> Self {
        Self {
            color,
            intensity,
            subbuffer: None,
        }
    }

    fn raw(&self) -> ambient_frag::UAmbientLightData {
        ambient_frag::UAmbientLightData {
            color: expand_vec3(&self.color),
            intensity: self.intensity.into(),
        }
    }
}

impl UniformSrc for AmbientLight {
    type UniformType = ambient_frag::UAmbientLightData;

    fn get_raw(&self) -> Self::UniformType {
        ambient_frag::UAmbientLightData {
            color: expand_vec3(&self.color),
            intensity: self.intensity.into(),
        }
    }
}

impl IntoPersistentUniform for AmbientLight {
    fn get_current_buffer(&self) -> Option<Subbuffer<Self::UniformType>> { self.subbuffer.clone() }
    fn set_current_buffer(&mut self, buf: Subbuffer<Self::UniformType>) { self.subbuffer = Some(buf) }
}


#[derive(Default, Clone)]
pub struct PointLight {
    position: Vec3,
    color: Vec3,
    intensity: f32,
    subbuffer: Option<Subbuffer<point_frag::UPointLightData>>,
}

impl PointLight {
    pub fn new(position: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            subbuffer: None,
        }
    }
}

impl UniformSrc for PointLight {
    type UniformType = point_frag::UPointLightData;

    fn get_raw(&self) -> Self::UniformType {
        point_frag::UPointLightData {
            position: expand_vec3(&self.position),
            color: expand_vec3(&self.color),
            intensity: self.intensity.into(),
        }
    }
}

#[cfg(feature = "mesh")]
impl IntoPersistentUniform for PointLight {
    fn get_current_buffer(&self) -> Option<Subbuffer<Self::UniformType>> { self.subbuffer.clone() }
    fn set_current_buffer(&mut self, buf: Subbuffer<Self::UniformType>) { self.subbuffer = Some(buf) }
}

#[cfg(feature = "marched")]
impl From<point_frag::UPointLightData> for marched_frag::UPointLight {
    fn from(value: point_frag::UPointLightData) -> Self {
        Self {
            color: value.color,
            intensity: value.intensity,
            position: value.position,
        }
    }
}