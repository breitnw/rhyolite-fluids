use std::sync::Arc;

use nalgebra_glm::{TMat4, perspective};
use vulkano::buffer::{cpu_pool::CpuBufferPoolSubbuffer, CpuBufferPool};
use winit::window::Window;

use crate::{transform::Transform, shaders::{albedo_vert, point_frag}, UnconfiguredError};
pub struct Camera {
    transform: Transform,

    view: TMat4<f32>,
    fovy: f32, 
    near_clipping_plane: f32, 
    far_clipping_plane: f32,

    needs_vp_update: bool,
    needs_pos_update: bool,

    post_config: Option<CameraPostConfig>,
}

struct CameraPostConfig {
    aspect_ratio: f32,
    projection: TMat4<f32>,
    vp_subbuffer: Option<Arc<CpuBufferPoolSubbuffer<albedo_vert::ty::VP_Data>>>,
    pos_subbuffer: Option<Arc<CpuBufferPoolSubbuffer<point_frag::ty::Camera_Data>>>
}// :)

impl Camera {
    /// Creates a new camera with a specified transform, FOV, and clipping planes.
    /// * `transform`: The transform to create the camera with, ignoring the scale field.
    /// * `fovy`: The camera's vertical field of view.
    /// * `near_clipping_plane`: The nearest distance at which geometry will clip out of view.
    /// * `far_clipping_plane`: The farthest distance at which geometry will clip out of view.
    pub fn new(
        mut transform: Transform,
        fovy: f32, 
        near_clipping_plane: f32, 
        far_clipping_plane: f32,
    ) -> Self {
        Camera {
            view: transform.get_rendering_matrices().0.try_inverse().unwrap(),
            transform,
            fovy,
            near_clipping_plane,
            far_clipping_plane,
            needs_vp_update: true,
            needs_pos_update: true,
            post_config: None,
        }
    }

    /// Configures the camera's aspect ratio based on the window size. Needs to be run before the camera can be used.
    pub fn configure(&mut self, window: &Window) {
        let dimensions: [i32; 2] = window.inner_size().into();
        let aspect_ratio = dimensions[0] as f32 / dimensions[1] as f32;
        let projection = perspective(aspect_ratio, self.fovy, self.near_clipping_plane, self.far_clipping_plane);
        
        self.needs_vp_update = true;
        self.needs_pos_update = true;

        self.post_config = Some(CameraPostConfig {
            aspect_ratio,
            projection,
            vp_subbuffer: None,
            pos_subbuffer: None,
        });
    }

    fn get_post_config(&self) -> Result<&CameraPostConfig, UnconfiguredError> {
        self.post_config.as_ref().ok_or(UnconfiguredError("Camera not yet configured. Do so with `Camera::configure()` before accessing projection matrix"))
    }

    fn get_post_config_mut(&mut self) -> Result<&mut CameraPostConfig, UnconfiguredError> {
        self.post_config.as_mut().ok_or(UnconfiguredError("Camera not yet configured. Do so with `Camera::configure()` before accessing projection matrix"))
    }

    pub(crate) fn is_configured(&self) -> bool {
        self.post_config.is_some()
    }

    /// Gets a mutable reference to the camera's transform.
    /// 
    /// Calling this function forces the camera's subbuffers to be updated at the end of the frame, so only use it when it's 
    /// necessary to move the camera. 
    pub fn transform_mut(&mut self) -> &mut Transform {
        self.needs_vp_update = true;
        self.needs_pos_update = true;
        &mut self.transform
    }

    /// Gets an immutable reference to the camera's transform. 
    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    pub(crate) fn get_vp_subbuffer(&mut self, vp_buffer_pool: &CpuBufferPool<albedo_vert::ty::VP_Data>) 
    -> Result<Arc<CpuBufferPoolSubbuffer<albedo_vert::ty::VP_Data>>, UnconfiguredError> {
        if self.needs_vp_update {
            self.needs_vp_update = false;
            self.view = self.transform.get_rendering_matrices().0.try_inverse().unwrap();
            self.get_post_config_mut()?.vp_subbuffer = Some(vp_buffer_pool.from_data(
                albedo_vert::ty::VP_Data {
                    view: self.view.into(),
                    projection: self.get_post_config()?.projection.into(),
                },
            ).unwrap());
        }
        Ok(self.get_post_config()?.vp_subbuffer.as_ref().unwrap().clone())
    }

    pub(crate) fn get_pos_subbuffer(&mut self, pos_buffer_pool: &CpuBufferPool<point_frag::ty::Camera_Data>)
    -> Result<Arc<CpuBufferPoolSubbuffer<point_frag::ty::Camera_Data>>, UnconfiguredError> {
        if self.needs_pos_update {
            self.needs_pos_update = false;
            let pos = self.transform().get_translation();
            self.get_post_config_mut()?.pos_subbuffer = Some(pos_buffer_pool.from_data(
                point_frag::ty::Camera_Data {
                    position: pos.into()
                },
            ).unwrap());
        }
        Ok(self.get_post_config()?.pos_subbuffer.as_ref().unwrap().clone())
    }
}
