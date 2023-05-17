use nalgebra_glm::Vec3;
use vulkano::buffer::{Buffer, BufferCreateInfo, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};
use vulkano::{buffer::BufferUsage, memory::allocator::MemoryAllocator};

use crate::{transform::Transform};

use crate::renderer::staging::{StagingBuffer, UniformSrc};
use crate::renderer::RenderBase;

/// Utilities for loading vertex and normal data from .obj files
pub mod loader;
pub use loader::{BasicVertex, UnlitVertex};

use loader::ModelBuilder;
use crate::shaders::albedo_vert;

/// Contains data that can only be generated after being configured with the Rhyolite instance
struct ObjectPostConfig {

}

pub struct MeshObjectBuilder {
    vertices: Vec<BasicVertex>,
    pub transform: Transform,
    specular_intensity: f32,
    shininess: f32,
}

impl MeshObjectBuilder {
    pub(crate) fn from_vertices(
        transform: Transform,
        vertices: Vec<BasicVertex>,
        specular_intensity: f32,
        shininess: f32,
    ) -> Self {
        Self {
            vertices,
            transform,
            specular_intensity,
            shininess,
        }
    }

    pub fn from_file(
        path: &'static str,
        translate: &Vec3,
        scale: &Vec3,
        color: &Vec3,
        specular: (f32, f32),
    ) -> MeshObjectBuilder {
        let vertices = ModelBuilder::from_file(path, true).build_basic([color.x, color.y, color.z]);
        let mut object_transform = Transform::zero();
        object_transform.set_translation(translate);
        object_transform.set_scale(scale);
        MeshObjectBuilder::from_vertices(object_transform, vertices, specular.0, specular.1)
    }

    pub(crate) fn build(
        self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        render_base: &RenderBase,
    ) -> MeshObject {
        let num_vertices = self.vertices.len();
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
                .into_iter(),
        )
            .unwrap()
            .into_device_local(num_vertices as u64, buffer_allocator, render_base);

        MeshObject {
            transform: self.transform,
            shininess: self.shininess,
            specular_intensity: self.specular_intensity,
            vertex_buffer,
        }
    }
}

/// An object, containing vertices and other data, that is rendered as a Mesh.
pub struct MeshObject {
    pub transform: Transform,
    specular_intensity: f32,
    shininess: f32,
    vertex_buffer: Subbuffer<[BasicVertex]>,
}

impl MeshObject {
    pub fn from_vertex_buffer(
        transform: Transform,
        vertex_buffer: Subbuffer<[BasicVertex]>,
        specular_intensity: f32,
        shininess: f32,
    ) -> Self {
        Self {
            transform,
            specular_intensity,
            shininess,
            vertex_buffer,
        }
    }

    pub(crate) fn get_vertex_buffer(&self) -> &Subbuffer<[BasicVertex]> {
        &self.vertex_buffer
    }
    pub(crate) fn get_specular(&self) -> (f32, f32) {
        (self.specular_intensity, self.shininess)
    }
}

impl UniformSrc for MeshObject {
    type UniformType = albedo_vert::UModelData;

    /// Gets the raw uniform data of this MeshObject, in the format of `albedo_vert::UModelData`.
    fn get_raw(&self) -> Self::UniformType {
        let (model_mat, normal_mat) = self.transform.get_matrices();

        albedo_vert::UModelData {
            model: model_mat.into(),
            normals: normal_mat.into(),
        }
    }
}
