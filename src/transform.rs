use nalgebra_glm::{TMat4, inverse_transpose, identity};

pub struct Transform{
    model: TMat4<f32>,
    normals: TMat4<f32>,
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    scale: TMat4<f32>,
    needs_update: bool,
}

impl Transform {
    pub fn zero() -> Self {
        Self{
            model: identity(),
            normals: identity(),
            translation: identity(),
            rotation: identity(),
            scale: identity(),
            needs_update: false,
        }
    }
    pub fn set_rotation_mat(&mut self, rotation: TMat4<f32>) {
        self.rotation = rotation;
        self.needs_update = true;
    }
    pub fn set_translation_mat(&mut self, translation: TMat4<f32>) {
        self.translation = translation;
        self.needs_update = true;
    }
    pub fn set_scale_mat(&mut self, scale: TMat4<f32>) {
        self.scale = scale;
        self.needs_update = true;
    }

    /// Gets the model and normal transformation matrices
    pub fn get_rendering_matrices(&mut self) -> (TMat4<f32>, TMat4<f32>) {
        if self.needs_update {
            // The model matrix is multiplied by a scaling matrix to invert the y-axis
            self.model = self.translation * self.rotation * self.scale;
            self.normals = inverse_transpose(self.model);
            self.needs_update = false;
        }
        (self.model, self.normals)
    }
}