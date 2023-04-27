use std::sync::Arc;
use nalgebra_glm::Vec3;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::device::Queue;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryUsage};

use crate::shaders::{ambient_frag, expand_vec3, point_frag};
use crate::renderer::staging::StagingBuffer;

// TODO: ideally make the get_buffer thing a trait

#[derive(Default, Clone)]
pub struct AmbientLight {
    color: Vec3,
    intensity: f32,
    subbuffer: Option<Subbuffer<ambient_frag::UAmbientLightData>>
}

impl AmbientLight {
    pub fn new(color: Vec3, intensity: f32) -> Self {
        Self {
            color,
            intensity,
            subbuffer: None,
        }
    }

    pub(crate) fn get_buffer(
        &mut self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        command_buffer_allocator: &StandardCommandBufferAllocator,
        transfer_queue: Arc<Queue>
    ) -> Subbuffer<ambient_frag::UAmbientLightData> {
        return if let Some(buffer) = self.subbuffer.as_ref() {
            buffer.clone()
        } else {
            let buf = Buffer::from_data(
                buffer_allocator,
                BufferCreateInfo {
                    usage: BufferUsage::TRANSFER_SRC,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    usage: MemoryUsage::Upload,
                    ..Default::default()
                },
                ambient_frag::UAmbientLightData {
                    color: expand_vec3(&self.color),
                    intensity: self.intensity.into(),
                }
            )
                .unwrap()
                .into_device_local(
                    buffer_allocator,
                    BufferUsage::UNIFORM_BUFFER,
                    command_buffer_allocator,
                    transfer_queue.clone()
                );
            self.subbuffer = Some(buf.clone());
            buf
        }
    }
}

#[derive(Default, Clone)]
pub struct PointLight {
    position: Vec3,
    color: Vec3,
    intensity: f32,
    subbuffer: Option<Subbuffer<point_frag::UPointLightData>>
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

    pub(crate) fn get_buffer(
        &mut self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        command_buffer_allocator: &StandardCommandBufferAllocator,
        transfer_queue: Arc<Queue>
    ) -> Subbuffer<point_frag::UPointLightData> {
        return if let Some(buffer) = self.subbuffer.as_ref() {
            buffer.clone()
        } else {
            let buf = Buffer::from_data(
                buffer_allocator,
                BufferCreateInfo {
                    usage: BufferUsage::TRANSFER_SRC,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    usage: MemoryUsage::Upload,
                    ..Default::default()
                },
                point_frag::UPointLightData {
                    position: expand_vec3(&self.position),
                    color: expand_vec3(&self.color),
                    intensity: self.intensity.into(),
                }
            )
                .unwrap()
                .into_device_local(
                    buffer_allocator,
                    BufferUsage::UNIFORM_BUFFER,
                    command_buffer_allocator,
                    transfer_queue.clone()
                );
            self.subbuffer = Some(buf.clone());
            buf
        }
    }
}
