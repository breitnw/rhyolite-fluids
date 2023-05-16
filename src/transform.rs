use std::cell::Cell;
use nalgebra_glm::{identity, inverse_transpose, scale, translate, vec3, Mat4, TMat4, Vec3};

pub struct Transform {
    translation: TMat4<f32>,
    rotation: TMat4<f32>,
    scale: TMat4<f32>,
    /// A cache containing the model (0) and normal (1) matrices
    cache: Cell<Option<(TMat4<f32>, TMat4<f32>)>>,
}

impl Transform {
    /// Gets a transform with default translation, rotation, and scale parameters.
    pub fn zero() -> Self {
        Self {
            cache: Cell::new(None),
            translation: identity(),
            rotation: identity(),
            scale: identity(),
        }
    }

    /// Uses a rotation matrix to set the rotation parameter of the transform.
    pub fn set_rotation_mat(&mut self, rotation: Mat4) {
        self.rotation = rotation;
        self.cache.set(None);
    }

    /// Uses a translation matrix to set the translation parameter of the transform.
    pub fn set_translation_mat(&mut self, translation: Mat4) {
        self.translation = translation;
        self.cache.set(None);
    }

    /// Uses a scale matrix to set the scale parameter of the transform.
    pub fn set_scale_mat(&mut self, scale: Mat4) {
        self.scale = scale;
        self.cache.set(None);
    }

    // TODO: function for set_rotation that takes quaternion
    // TODO: potentially store vec3s and quaternions for later access, and generate all matrices in get_rendering_matrices

    /// Uses a Vec3 to set the translation parameter of the transform.
    pub fn set_translation(&mut self, val: &Vec3) {
        self.translation = translate(&identity(), val);
        self.cache.set(None);
    }

    /// Uses a Vec3 to set the scale parameter of the transform.
    pub fn set_scale(&mut self, val: &Vec3) {
        self.scale = scale(&identity(), val);
        self.cache.set(None);
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

    /// Updates the model and normal transformation matrices if there has been a change since the
    /// last time this function was called, and then returns these updated matrices.
    pub fn get_matrices(&self) -> (TMat4<f32>, TMat4<f32>) {
        if let Some(cache) = self.cache.get() {
            return cache;
        }
        let model = self.translation * self.rotation * self.scale;
        let normal =  inverse_transpose(model);
        self.cache.set(Some((model, normal)));

        (model, normal)
    }
}
