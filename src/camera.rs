use std::sync::Arc;

use nalgebra_glm::{TMat4, perspective};
use vulkano::{descriptor_set::{allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet}, buffer::{CpuAccessibleBuffer, BufferUsage, cpu_pool::CpuBufferPoolSubbuffer, CpuBufferPool}};
use winit::window::Window;

use crate::{transform::Transform, shaders::deferred_vert, UnconfiguredError};
pub struct Camera {
    transform: Transform,

    view: TMat4<f32>,
    fovy: f32, 
    near_clipping_plane: f32, 
    far_clipping_plane: f32,
    needs_update: bool,

    config: Option<CameraConfig>,   
}

struct CameraConfig {
    aspect_ratio: f32,
    projection: TMat4<f32>,
    vp_subbuffer: Option<Arc<CpuBufferPoolSubbuffer<deferred_vert::ty::VP_Data>>>,
}

impl Camera {
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
            needs_update: true,
            config: None,
        }
    }

    pub fn configure(&mut self, window: &Window) {
        let dimensions: [i32; 2] = window.inner_size().into();
        let aspect_ratio = dimensions[0] as f32 / dimensions[1] as f32;
        let projection = perspective(aspect_ratio, self.fovy, self.near_clipping_plane, self.far_clipping_plane);
        
        self.needs_update = true;
        self.config = Some(CameraConfig {
            aspect_ratio,
            projection,
            vp_subbuffer: None,
        });
    }

    fn get_config(&self) -> Result<&CameraConfig, UnconfiguredError> {
        self.config.as_ref().ok_or(UnconfiguredError("Camera not yet configured. Do so with `Camera::configure()` before accessing projection matrix"))
    }

    fn get_config_mut(&mut self) -> Result<&mut CameraConfig, UnconfiguredError> {
        self.config.as_mut().ok_or(UnconfiguredError("Camera not yet configured. Do so with `Camera::configure()` before accessing projection matrix"))
    }

    pub(crate) fn is_configured(&self) -> bool {
        self.config.is_none()
    }

    pub(crate) fn get_vp_descriptor_set(
        &mut self,
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        descriptor_set_layout: &Arc<DescriptorSetLayout>,
        vp_buffer_pool: &CpuBufferPool<deferred_vert::ty::VP_Data>
    ) -> Result<Arc<PersistentDescriptorSet>, UnconfiguredError> {
        if self.needs_update {
            self.needs_update = false;
            self.view = self.transform.get_rendering_matrices().0.try_inverse().unwrap();
            self.get_config_mut()?.vp_subbuffer = Some(vp_buffer_pool.from_data(
                deferred_vert::ty::VP_Data {
                    view: self.view.into(),
                    projection: self.get_config()?.projection.into(),
                },
            ).unwrap());
        }
        
        // TODO: use a different descriptor set because PersistentDescriptorSet is expected to be long-lived
        Ok(PersistentDescriptorSet::new(
            descriptor_set_allocator,
            descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, self.get_config()?.vp_subbuffer.as_ref().unwrap().clone()),
            ],
        ).unwrap())
    }


    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }
    pub fn transform(&self) -> &Transform {
        &self.transform
    }
}