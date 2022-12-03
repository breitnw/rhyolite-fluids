use nalgebra_glm::{TMat4, inverse_transpose, identity, vec3, scaling};

pub struct Transform{
    model: TMat4<f32>,
    normals: TMat4<f32>,
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    scale: TMat4<f32>,
    update_required: bool,
}

impl Transform {
    pub fn new() -> Self {
        Self{
            model: identity(),
            normals: identity(),
            translation: identity(),
            rotation: identity(),
            scale: identity(),
            update_required: false,
        }
    }
    pub fn set_rotation_mat(&mut self, rotation: TMat4<f32>) {
        self.rotation = rotation;
        self.update_required = true;
    }
    pub fn set_translation_mat(&mut self, translation: TMat4<f32>) {
        self.translation = translation;
        self.update_required = true;
    }
    pub fn set_scale_mat(&mut self, scale: TMat4<f32>) {
        self.scale = scale;
        self.update_required = true;
    }

    /// Gets the model and normal transformation matrices
    pub fn get_rendering_matrices(&mut self) -> (TMat4<f32>, TMat4<f32>) {
        if self.update_required {
            // The model matrix is multiplied by a scaling matrix to invert the y-axis
            self.model = self.translation * self.rotation * self.scale;
            self.normals = inverse_transpose(self.model);
            self.update_required = false;
        }
        (self.model, self.normals)
    }
}