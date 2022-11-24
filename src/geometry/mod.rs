use std::sync::Arc;

use vulkano::{buffer::{CpuAccessibleBuffer, BufferUsage}, memory::allocator::MemoryAllocator};

use crate::{transform::Transform, UnconfiguredError};

pub mod loader;
use loader::Vertex;
vulkano::impl_vertex!(Vertex, position, normal, color);

pub mod dummy;
use dummy::DummyVertex;
vulkano::impl_vertex!(DummyVertex, position);


struct ObjectConfig {
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
}
pub struct Object {
    vertices: Option<Vec<Vertex>>,
    pub transform: Transform,

    config: Option<ObjectConfig>,
}

impl Object {
    pub fn new(transform: Transform, vertices: Vec<Vertex>) -> Self {
        Self {
            vertices: Some(vertices),
            transform,
            config: None,
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
        self.config = Some(ObjectConfig {
            vertex_buffer
        });
    }

    fn get_config(&self) -> Result<&ObjectConfig, UnconfiguredError> {
        self.config.as_ref().ok_or(UnconfiguredError("Object not properly configured"))
    }

    pub(crate) fn vertex_buffer(&self) -> Result<&Arc<CpuAccessibleBuffer<[Vertex]>>, UnconfiguredError> {
        Ok(&self.get_config()?.vertex_buffer)
    }
}