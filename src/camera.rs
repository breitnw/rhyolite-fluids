use std::sync::Arc;

use nalgebra_glm::{TMat4, perspective};
use vulkano::{memory::allocator::MemoryAllocator, descriptor_set::{allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet}, buffer::{CpuAccessibleBuffer, BufferUsage}};
use winit::window::Window;

use crate::{transform::Transform, shaders::deferred_vert, UnconfiguredError};
pub struct Camera {
    view: TMat4<f32>,
    fovy: f32, 
    near_clipping_plane: f32, 
    far_clipping_plane: f32,

    config: Option<CameraConfig>,
}

struct CameraConfig {
    aspect_ratio: f32,
    projection: TMat4<f32>,
    vp_buffer: Arc<CpuAccessibleBuffer<deferred_vert::ty::VP_Data>>,
}

impl Camera {
    pub fn new(
        mut transform: Transform,
        fovy: f32, 
        near_clipping_plane: f32, 
        far_clipping_plane: f32,
    ) -> Self {
        Camera {
            view: transform.get_matrices().0.try_inverse().unwrap(),
            fovy,
            near_clipping_plane,
            far_clipping_plane,
            config: None,
        }
    }

    pub fn configure(&mut self, window: &Window, buffer_allocator:&(impl MemoryAllocator + ?Sized)) {
        let dimensions: [i32; 2] = window.inner_size().into();
        let aspect_ratio = dimensions[0] as f32 / dimensions[1] as f32;
        let projection = perspective(aspect_ratio, self.fovy, self.near_clipping_plane, self.far_clipping_plane);
        let vp_buffer = CpuAccessibleBuffer::from_data(
            buffer_allocator, 
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            }, 
            false, 
            deferred_vert::ty::VP_Data {
                view: self.view().into(),
                projection: projection.into(),
            },
        ).unwrap();
        self.config = Some(CameraConfig {
            aspect_ratio,
            projection,
            vp_buffer,
        });
    }

    fn get_config(&self) -> Result<&CameraConfig, UnconfiguredError> {
        self.config.as_ref().ok_or(UnconfiguredError("Camera not yet configured. Do so with `Camera::configure()` before accessing projection matrix"))
    }

    pub fn vp_buffer(&self) -> Result<TMat4<f32>, UnconfiguredError> {
        let config = self.get_config()?;
        Ok(config.projection)
    }
    
    pub fn proj(&self) -> Result<&Arc<CpuAccessibleBuffer<deferred_vert::ty::VP_Data>>, UnconfiguredError> {
        Ok(&self.get_config()?.vp_buffer)
    }

    pub fn view(&self) -> TMat4<f32> {
        self.view
    }

    pub(crate) fn get_vp_descriptor_set(
        &self,
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        descriptor_set_layout: &Arc<DescriptorSetLayout>,
    ) -> Result<Arc<PersistentDescriptorSet>, UnconfiguredError> {
        let vp_buffer = self.proj()?;
        
        // TODO: use a different descriptor set because PersistentDescriptorSet is expected to be long-lived
        Ok(PersistentDescriptorSet::new(
            descriptor_set_allocator,
            descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, vp_buffer.clone()),
            ],
        ).unwrap())
    }
}