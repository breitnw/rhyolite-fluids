use std::sync::Arc;

use vulkano::{buffer::{CpuAccessibleBuffer, BufferUsage}, memory::allocator::MemoryAllocator};

use crate::{transform::Transform, UnconfiguredError};

pub mod loader;
use loader::ColorVertex;
vulkano::impl_vertex!(ColorVertex, position, normal, color);

pub mod dummy;
use dummy::DummyVertex;
vulkano::impl_vertex!(DummyVertex, position);


/// Contains data that can only be generated after being configured with the Rhyolite instance
struct ObjectPostConfig {
    vertex_buffer: Arc<CpuAccessibleBuffer<[ColorVertex]>>,
    // index buffer should go here
}
pub struct Object {
    vertices: Option<Vec<ColorVertex>>,
    pub transform: Transform,

    post_config: Option<ObjectPostConfig>,
}

impl Object {
    pub fn new(transform: Transform, vertices: Vec<ColorVertex>) -> Self {
        Self {
            vertices: Some(vertices),
            transform,
            post_config: None,
        }
    }

    pub fn configure(&mut self, vb_allocator: &(impl MemoryAllocator + ?Sized)) {
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            vb_allocator, 
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            }, 
            false,
            self.vertices.take().expect("Object already configured").into_iter(),
        ).unwrap();
        self.post_config = Some(ObjectPostConfig {
            vertex_buffer
        });
    }

    fn get_post_config(&self) -> Result<&ObjectPostConfig, UnconfiguredError> {
        self.post_config.as_ref().ok_or(UnconfiguredError("Object not properly configured"))
    }

    pub(crate) fn vertex_buffer(&self) -> Result<&Arc<CpuAccessibleBuffer<[ColorVertex]>>, UnconfiguredError> {
        Ok(&self.get_post_config()?.vertex_buffer)
    }
}