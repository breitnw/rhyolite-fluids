use std::sync::Arc;

use vulkano::{buffer::{CpuAccessibleBuffer, BufferUsage}, memory::allocator::MemoryAllocator};

use crate::transform::Transform;

pub mod loader;
use loader::Vertex;
vulkano::impl_vertex!(Vertex, position, normal, color);

pub mod dummy;
use dummy::DummyVertex;
vulkano::impl_vertex!(DummyVertex, position);

pub struct Object {
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub transform: Transform,
}

impl Object {
    pub fn new(transform: Transform, vertices: Vec<Vertex>, vb_allocator: &(impl MemoryAllocator + ?Sized)) -> Self {
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            vb_allocator, 
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            }, 
            false,
            vertices.into_iter(),
        ).unwrap();
        Self {
            vertex_buffer,
            transform,
        }
    }

    pub(crate) fn vertex_buffer(&self) -> &Arc<CpuAccessibleBuffer<[Vertex]>> {
        &self.vertex_buffer
    }
}

pub fn cube(allocator: &(impl MemoryAllocator + ?Sized)) -> Arc<CpuAccessibleBuffer<[Vertex]>> {
    CpuAccessibleBuffer::from_iter(
        allocator,
        BufferUsage {
            vertex_buffer: true,
            ..Default::default()
        }, 
        false, 
        [
            // front face
            Vertex { position: [-1.000000, -1.000000, 1.000000], normal: [0.0000, 0.0000, 1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, 1.000000], normal: [0.0000, 0.0000, 1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, 1.000000], normal: [0.0000, 0.0000, 1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, -1.000000, 1.000000], normal: [0.0000, 0.0000, 1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, 1.000000], normal: [0.0000, 0.0000, 1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, -1.000000, 1.000000], normal: [0.0000, 0.0000, 1.0000], color: [1.0, 0.35, 0.137]},

            // back face
            Vertex { position: [1.000000, -1.000000, -1.000000], normal: [0.0000, 0.0000, -1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, -1.000000], normal: [0.0000, 0.0000, -1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, -1.000000], normal: [0.0000, 0.0000, -1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, -1.000000, -1.000000], normal: [0.0000, 0.0000, -1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, -1.000000], normal: [0.0000, 0.0000, -1.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, -1.000000, -1.000000], normal: [0.0000, 0.0000, -1.0000], color: [1.0, 0.35, 0.137]},

            // top face
            Vertex { position: [-1.000000, -1.000000, 1.000000], normal: [0.0000, -1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, -1.000000, 1.000000], normal: [0.0000, -1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, -1.000000, -1.000000], normal: [0.0000, -1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, -1.000000, 1.000000], normal: [0.0000, -1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, -1.000000, -1.000000], normal: [0.0000, -1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, -1.000000, -1.000000], normal: [0.0000, -1.0000, 0.0000], color: [1.0, 0.35, 0.137]},

            // bottom face
            Vertex { position: [1.000000, 1.000000, 1.000000], normal: [0.0000, 1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, 1.000000], normal: [0.0000, 1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, -1.000000], normal: [0.0000, 1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, 1.000000], normal: [0.0000, 1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, -1.000000], normal: [0.0000, 1.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, -1.000000], normal: [0.0000, 1.0000, 0.0000], color: [1.0, 0.35, 0.137]},

            // left face
            Vertex { position: [-1.000000, -1.000000, -1.000000], normal: [-1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, -1.000000], normal: [-1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, 1.000000], normal: [-1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, -1.000000, -1.000000], normal: [-1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, 1.000000, 1.000000], normal: [-1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [-1.000000, -1.000000, 1.000000], normal: [-1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},

            // right face
            Vertex { position: [1.000000, -1.000000, 1.000000], normal: [1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, 1.000000], normal: [1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, -1.000000], normal: [1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, -1.000000, 1.000000], normal: [1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, 1.000000, -1.000000], normal: [1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
            Vertex { position: [1.000000, -1.000000, -1.000000], normal: [1.0000, 0.0000, 0.0000], color: [1.0, 0.35, 0.137]},
        ].into_iter()
    ).unwrap()
}