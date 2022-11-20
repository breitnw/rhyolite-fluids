use std::sync::Arc;

use nalgebra_glm::{TMat4, perspective};
use vulkano::{memory::allocator::MemoryAllocator, descriptor_set::{allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet}, buffer::{CpuAccessibleBuffer, BufferUsage}};

use crate::{transform::Transform, shaders::deferred_vert};
pub struct Camera {
    view: TMat4<f32>,
    projection: TMat4<f32>,

    fovy: f32, 
    near_clipping_plane: f32, 
    far_clipping_plane: f32,
}

impl Camera {
    pub fn new(
        mut transform: Transform, 
        render_size: [u32; 2], 
        fovy: f32, 
        near_clipping_plane: f32, 
        far_clipping_plane: f32
    ) -> Self {
        let aspect_ratio = render_size[0] as f32 / render_size[1] as f32;
        Camera {
            view: transform.get_matrices().0.try_inverse().unwrap(),
            projection: perspective(aspect_ratio, fovy, near_clipping_plane, far_clipping_plane),
            fovy,
            near_clipping_plane,
            far_clipping_plane,
        }
    }
    
    pub fn proj(&self) -> TMat4<f32> {
        self.projection
    }

    pub fn view(&self) -> TMat4<f32> {
        self.view
    }

    pub fn update_aspect_ratio(&mut self, render_size: [f32; 2]) {
        let aspect_ratio = render_size[0] as f32 / render_size[1] as f32;
        self.projection = perspective(aspect_ratio, self.fovy, self.near_clipping_plane, self.far_clipping_plane);
    }

    pub(crate) fn get_vp_descriptor_set(
        &self,
        buffer_allocator:&(impl MemoryAllocator + ?Sized), 
        descriptor_set_allocator: &StandardDescriptorSetAllocator,
        descriptor_set_layout: &Arc<DescriptorSetLayout>,
    ) -> Arc<PersistentDescriptorSet> {
        let new_vp_buffer = CpuAccessibleBuffer::from_data(
            buffer_allocator, 
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            }, 
            false, 
            deferred_vert::ty::VP_Data {
                view: self.view().into(),
                projection: self.proj().into(),
            },
        ).unwrap();
        
        // TODO: use a different descriptor set because PersistentDescriptorSet is expected to be long-lived
        PersistentDescriptorSet::new(
            descriptor_set_allocator,
            descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, new_vp_buffer.clone()),
            ],
        ).unwrap()
    }
}