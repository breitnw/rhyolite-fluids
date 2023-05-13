use nalgebra_glm::{identity, inverse_transpose, scale, translate, vec3, Mat4, TMat4, Vec3};

pub struct Transform {
    model: TMat4<f32>,
    normals: TMat4<f32>,
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    scale: TMat4<f32>,
    needs_update: bool,
}

impl Transform {
    /// Gets a transform with default translation, rotation, and scale parameters.
    pub fn zero() -> Self {
        Self {
            model: identity(),
            normals: identity(),
            translation: identity(),
            rotation: identity(),
            scale: identity(),
            needs_update: false,
        }
    }

    /// Uses a rotation matrix to set the rotation parameter of the transform.
    pub fn set_rotation_mat(&mut self, rotation: Mat4) {
        self.rotation = rotation;
        self.needs_update = true;
    }

    /// Uses a translation matrix to set the translation parameter of the transform.
    pub fn set_translation_mat(&mut self, translation: Mat4) {
        self.translation = translation;
        self.needs_update = true;
    }

    /// Uses a scale matrix to set the scale parameter of the transform.
    pub fn set_scale_mat(&mut self, scale: Mat4) {
        self.scale = scale;
        self.needs_update = true;
    }

    // TODO: function for set_rotation that takes quaternion
    // TODO: potentially store vec3s and quaternions for later access, and generate all matrices in get_rendering_matrices

    /// Uses a Vec3 to set the translation parameter of the transform.
    pub fn set_translation(&mut self, val: &Vec3) {
        self.translation = translate(&identity(), val);
        self.needs_update = true;
    }

    /// Uses a Vec3 to set the scale parameter of the transform.
    pub fn set_scale(&mut self, val: &Vec3) {
        self.scale = scale(&identity(), val);
        self.needs_update = true;
    }

    pub fn get_translation(&self) -> Vec3 {
        vec3(
            self.translation[12],
            self.translation[13],
            self.translation[14],
        )
    }
    pub fn get_rotation_mat(&self) -> Mat4 {
        self.rotation
    }

    /// Updates the model and normal transformation matrices
    pub(crate) fn update_matrices(&mut self) {
        if self.needs_update {
            // The model matrix is multiplied by a scaling matrix to invert the y-axis
            self.model = self.translation * self.rotation * self.scale;
            self.normals = inverse_transpose(self.model);
            self.needs_update = false;
        }
    }

    /// Gets the model and normal transformation matrices. If there has been a change since the
    /// last time `get_updated_matrices()` or `update_matrices` has been called, it will **NOT**
    /// be reflected.
    pub(crate) fn get_current_matrices(&self) -> (TMat4<f32>, TMat4<f32>) {
        (self.model, self.normals)
    }

    /// Updates the model and normal transformation matrices if there has been a change since the
    /// last time this function was called, and then returns these updated matrices.
    pub fn get_matrices(&mut self) -> (TMat4<f32>, TMat4<f32>) {
        self.update_matrices();
        self.get_current_matrices()
    }
}
