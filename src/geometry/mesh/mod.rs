use nalgebra_glm::Vec3;
use vulkano::buffer::{Buffer, BufferCreateInfo, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage};
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::{buffer::BufferUsage, memory::allocator::MemoryAllocator};

use crate::{transform::Transform};

use crate::renderer::staging::{StagingBuffer, UniformSrc};
use crate::renderer::{RenderBase, Renderer};

/// Utilities for loading vertex and normal data from .obj files
pub mod loader;
pub use loader::{BasicVertex, UnlitVertex};

use loader::ModelBuilder;
use crate::renderer::mesh::MeshRenderer;
use crate::shaders::{albedo_vert, albedo_frag};

pub struct MeshObjectBuilder<T: Vertex> {
    vertices: Vec<T>,
    pub transform: Transform,
    specular_intensity: f32,
    shininess: f32,
}

impl MeshObjectBuilder<BasicVertex> {
    pub fn from_file(
        path: &'static str,
        translate: &Vec3,
        scale: &Vec3,
        color: &Vec3,
        specular: (f32, f32),
    ) -> MeshObjectBuilder<BasicVertex> {
        let vertices = ModelBuilder::from_file(path, true).build_basic([color.x, color.y, color.z]);
        let mut object_transform = Transform::identity();
        object_transform.set_translation(translate);
        object_transform.set_scale(scale);
        MeshObjectBuilder::from_vertices(object_transform, vertices, specular.0, specular.1)
    }
}

impl<T: Vertex> MeshObjectBuilder<T> {
    pub(crate) fn from_vertices(
        transform: Transform,
        vertices: Vec<T>,
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

    pub fn build(self, renderer: &MeshRenderer) -> MeshObject<T> {
        let buffer_allocator = renderer.get_buffer_allocator();
        let base = renderer.get_base();

        let num_vertices = self.vertices.len();
        let vertex_buffer = Buffer::from_iter(
            &buffer_allocator,
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
            .into_device_local(num_vertices as u64, &buffer_allocator, &base);

        MeshObject::from_vertex_buffer(
            self.transform, 
            vertex_buffer, 
            self.specular_intensity, 
            self.shininess
        )
    }
}

pub struct MeshObjectParams {
    pub specular_intensity: f32,
    pub shininess: f32,
    pub transform: Transform,
}

impl UniformSrc<albedo_vert::UModelData> for MeshObjectParams {
    /// Gets the raw uniform data of this MeshObject, in the format of `albedo_vert::UModelData`.
    fn get_raw(&self) -> albedo_vert::UModelData {
        let (model_mat, normal_mat) = self.transform.get_matrices();

        albedo_vert::UModelData {
            model: model_mat.into(),
            normals: normal_mat.into(),
        }
    }
}

impl UniformSrc<albedo_frag::USpecularData> for MeshObjectParams {
    /// Gets the raw uniform data of this MeshObject, in the format of `albedo_vert::UModelData`.
    fn get_raw(&self) -> albedo_frag::USpecularData {
        albedo_frag::USpecularData { 
            intensity: self.specular_intensity,
            shininess: self.shininess,
        }
    }
}

/// An object, containing vertices and other data, that is rendered as a Mesh.
pub struct MeshObject<T: Vertex> {
    vertex_buffer: Subbuffer<[T]>,
    params: MeshObjectParams 
}

impl<T: Vertex> MeshObject<T> {
    pub fn from_vertex_buffer(
        transform: Transform,
        vertex_buffer: Subbuffer<[T]>,
        specular_intensity: f32,
        shininess: f32,
    ) -> Self {
        Self {
            params: MeshObjectParams {
                transform,
                specular_intensity,
                shininess,
            },
            vertex_buffer,
        }
    }

    pub(crate) fn vertex_buffer(&self) -> &Subbuffer<[T]> {
        &self.vertex_buffer
    }
    pub(crate) fn params(&self) -> &MeshObjectParams {
        &self.params
    }
    pub fn transform(&self) -> &Transform {
        &self.params.transform
    }
    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.params.transform
    }
}