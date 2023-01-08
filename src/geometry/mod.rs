use std::sync::Arc;

use vulkano::{buffer::{CpuAccessibleBuffer, BufferUsage}, memory::allocator::MemoryAllocator};

use crate::{transform::Transform, UnconfiguredError};

pub mod loader;
use loader::BasicVertex;
use loader::UnlitVertex;
vulkano::impl_vertex!(BasicVertex, position, normal, color);
vulkano::impl_vertex!(UnlitVertex, position, color);

pub mod dummy;
use dummy::DummyVertex;
vulkano::impl_vertex!(DummyVertex, position);


/// Contains data that can only be generated after being configured with the Rhyolite instance
struct ObjectPostConfig {
    vertex_buffer: Arc<CpuAccessibleBuffer<[BasicVertex]>>,
}
pub struct MeshObject {
    vertices: Option<Vec<BasicVertex>>,
    pub transform: Transform,
    specular_intensity: f32,
    shininess: f32,

    post_config: Option<ObjectPostConfig>,
}

impl MeshObject {
    pub fn new(transform: Transform, vertices: Vec<BasicVertex>, specular_intensity: f32, shininess: f32) -> Self {
        Self {
            vertices: Some(vertices),
            transform,
            post_config: None,
            specular_intensity,
            shininess,
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

    pub(crate) fn get_vertex_buffer(&self) -> Result<&Arc<CpuAccessibleBuffer<[BasicVertex]>>, UnconfiguredError> {
        Ok(&self.get_post_config()?.vertex_buffer)
    }

    pub(crate) fn get_specular(&self) -> (f32, f32) {
        (self.specular_intensity, self.shininess)
    }
}

pub struct MarchedObject {
    // TODO: implement, move to another file
}