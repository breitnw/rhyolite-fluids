use nalgebra_glm::Vec3;
use vulkano::buffer::{Buffer, BufferCreateInfo, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};
use vulkano::{buffer::BufferUsage, memory::allocator::MemoryAllocator};

use crate::{transform::Transform, UnconfiguredError};

pub mod loader;
use loader::BasicVertex;
use crate::renderer::RenderBase;
use crate::renderer::staging::StagingBuffer;

pub mod dummy;

use self::loader::ModelBuilder;

pub mod marched;

/// Contains data that can only be generated after being configured with the Rhyolite instance
struct ObjectPostConfig {
    vertex_buffer: Subbuffer<[BasicVertex]>,
}

/// An object, containing vertices and other data, that is rendered as a mesh.
pub struct MeshObject {
    vertices: Option<Vec<BasicVertex>>,
    pub transform: Transform,
    specular_intensity: f32,
    shininess: f32,

    post_config: Option<ObjectPostConfig>,
}

impl MeshObject {
    pub(crate) fn new(
        transform: Transform,
        vertices: Vec<BasicVertex>,
        specular_intensity: f32,
        shininess: f32,
    ) -> Self {
        Self {
            vertices: Some(vertices),
            transform,
            post_config: None,
            specular_intensity,
            shininess,
        }
    }

    pub(crate) fn configure(
        &mut self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        render_base: &RenderBase
    ) {
        let num_vertices = self.vertices.as_ref().map(|v| v.len());
        let vertex_buffer = Buffer::from_iter(
            buffer_allocator,
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC | BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            self.vertices
                .take()
                .expect("Object already configured")
                .into_iter(),
        )
            .unwrap()
            .into_device_local(
                num_vertices.unwrap() as u64,
                buffer_allocator,
                render_base,
            );
        self.post_config = Some(ObjectPostConfig { vertex_buffer });
    }

    fn get_post_config(&self) -> Result<&ObjectPostConfig, UnconfiguredError> {
        self.post_config
            .as_ref()
            .ok_or(UnconfiguredError("Object not properly configured"))
    }

    pub(crate) fn get_vertex_buffer(&self) -> Result<&Subbuffer<[BasicVertex]>, UnconfiguredError> {
        Ok(&self.get_post_config()?.vertex_buffer)
    }

    pub(crate) fn get_specular(&self) -> (f32, f32) {
        (self.specular_intensity, self.shininess)
    }

    pub fn from_file(
        path: &'static str,
        translate: &Vec3,
        scale: &Vec3,
        color: &Vec3,
        specular: (f32, f32),
    ) -> MeshObject {
        let vertices = ModelBuilder::from_file(path, true).build_basic([color.x, color.y, color.z]);
        let mut object_transform = Transform::zero();
        object_transform.set_translation(translate);
        object_transform.set_scale(scale);
        MeshObject::new(object_transform, vertices, specular.0, specular.1)
    }
}
