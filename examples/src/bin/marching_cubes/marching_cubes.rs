use std::sync::Arc;
use rhyolite::{renderer::{mesh::MeshRenderer, Renderer}, geometry::{mesh::MeshObjectParams, marched::Metaball}};
use vulkano::{
    buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
    buffer::BufferUsage,
    command_buffer::DrawIndirectCommand,
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::{
        ComputePipeline,
        Pipeline,
        GraphicsPipeline,
        graphics::{
            input_assembly::InputAssemblyState,
            viewport::{ViewportState, Viewport},
            depth_stencil::DepthStencilState,
            rasterization::{RasterizationState, CullMode}
        }
    },
    render_pass::Subpass,
};
use vulkano::buffer::{Buffer, BufferCreateInfo, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryUsage};
use vulkano::padded::Padded;
use vulkano::pipeline::PipelineBindPoint;
use rhyolite::renderer::RenderBase;
use rhyolite::renderer::staging::StagingBuffer;

use crate::metaball;

const GRID_SIZE: [u32; 3] = [64, 64, 64];
const MAX_VERTICES_PER_THREAD: u32 = 5;

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "src/bin/marching_cubes/shaders/marching_cubes.comp"
    }
}
mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/bin/marching_cubes/shaders/marching_cubes.frag"
    }
}
mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/bin/marching_cubes/shaders/marching_cubes.vert"
    }
}

pub struct MarchingCubesGenerator {
    indirect_args_pool: SubbufferAllocator,
    vertex_pool: SubbufferAllocator,
    compute_pipeline: Arc<ComputePipeline>,
    graphics_pipeline: Arc<GraphicsPipeline>,
    index_descriptors: Arc<PersistentDescriptorSet>,
}

impl MarchingCubesGenerator {
    pub fn new(renderer: &MeshRenderer) -> Self {
        let buffer_allocator = renderer.get_buffer_allocator();
        let render_base = renderer.get_base();
        let device = render_base.get_device();

        // Create descriptor pools for the indirect and vertex buffers
        let vertex_pool = SubbufferAllocator::new(
            buffer_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                // DOESN'T HAVE BufferUsage::VERTEX_BUFFER!!! FAKE VERTEX BUFFER, ONLY USE IT AS A
                // STORAGE BUFFER!!!!
                buffer_usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
        );
        let indirect_args_pool = SubbufferAllocator::new(
            buffer_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::INDIRECT_BUFFER | BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
        );

        // Create the compute and graphics pipelines
        let cs = cs::load(device.clone()).unwrap();
        let compute_pipeline = ComputePipeline::new(
            device.clone(),
            cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {}
        ).unwrap();

        let graphics_pipeline = create_graphics_pipeline(renderer);

        // Load a descriptor set for index data used in the marching cubes compute shader, based on
        // the data from `triangle_counts.txt` and `vertex_edge_indices.txt`.
        let polygon_counts = get_u32_buf(
            include_str!("render_data/triangle_counts.txt"),
            &buffer_allocator,
            render_base
        );
        let polygon_edge_indices = get_u32_buf(
            include_str!("render_data/vertex_edge_indices.txt"),
            &buffer_allocator,
            render_base
        );
        let index_descriptors = PersistentDescriptorSet::new(
            &renderer.get_descriptor_set_allocator(),
            compute_pipeline.layout().set_layouts().get(1).unwrap().clone(),
            [
                WriteDescriptorSet::buffer(0, polygon_counts),
                WriteDescriptorSet::buffer(1, polygon_edge_indices),
            ]
        ).unwrap();

        Self {
            indirect_args_pool,
            vertex_pool,
            compute_pipeline,
            graphics_pipeline,
            index_descriptors,
        }
    }

    /// Create the indirect buffer, used to keep track of the number of vertices that have been initialized
    pub fn indirect_buffer(&self) -> Subbuffer<[DrawIndirectCommand]> {
        let indirect_commands = [DrawIndirectCommand {
            vertex_count: 0,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        }];
        let indirect_buffer = self.indirect_args_pool
            .allocate_slice(indirect_commands.len() as u64)
            .unwrap();
        indirect_buffer
            .write()
            .unwrap()
            .copy_from_slice(&indirect_commands);
        indirect_buffer
    }

    /// Create a buffer for vertex data, zeroed for initialization
    pub fn vertex_buffer(&self) -> Subbuffer<[[f32; 4]]> {
        const NUM_THREADS: u32 = GRID_SIZE[0] * GRID_SIZE[1] * GRID_SIZE[2];

        // The number of vertices = vec4s per vertex * maximum possible vertices per thread * number of threads
        let vertex_iter = (0..(3 * MAX_VERTICES_PER_THREAD * NUM_THREADS)).map(|_| [0.0; 4]);
        let vertex_buffer = self.vertex_pool.allocate_slice(vertex_iter.len() as u64).unwrap();
        for (entry, data) in vertex_buffer.write().unwrap().iter_mut().zip(vertex_iter) {
            *entry = data
        }
        vertex_buffer
    }

    /// Gets the `GraphicsPipeline` associated with this `MarchingCubesGenerator`.
    pub fn graphics_pipeline(&self) -> &Arc<GraphicsPipeline> {
        &self.graphics_pipeline
    }

    pub fn recreate_graphics_pipeline(&mut self, renderer: &MeshRenderer) {
        self.graphics_pipeline = create_graphics_pipeline(renderer);
    }

    /// Gets the descriptor data for the compute pipeline
    pub fn compute_descriptors(
        &self,
        renderer: &MeshRenderer,
        vertex_buffer: Subbuffer<[[f32; 4]]>,
        indirect_buffer: Subbuffer<[DrawIndirectCommand]>,
        objects: &Vec<Metaball>,
    ) -> (Arc<PersistentDescriptorSet>, Arc<PersistentDescriptorSet>, Arc<PersistentDescriptorSet>) {
        let set_layouts = self.compute_pipeline.layout().set_layouts();

        let layout = set_layouts.get(0).unwrap();
        let sbo_set = PersistentDescriptorSet::new(
            &renderer.get_descriptor_set_allocator(),
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, vertex_buffer.clone()),
                WriteDescriptorSet::buffer(1, indirect_buffer.clone()),
            ]
        ).unwrap();

        let metaball_set = metaball::metaball_set(
            renderer, 
            objects, 
            self.compute_pipeline.layout().set_layouts().get(2).unwrap().clone()
        );

        (sbo_set, self.index_descriptors.clone(), metaball_set)
    }

    /// Gets descriptor data for the render pipeline. Doesn't get all of the render descriptors, 
    /// just the vertex buffer with the correct binding
    pub fn graphics_descriptors(
        &self,
        vertex_buffer: Subbuffer<[[f32; 4]]>,
        renderer: &MeshRenderer,
        params: &MeshObjectParams
    ) -> (Arc<PersistentDescriptorSet>, Arc<PersistentDescriptorSet>, Arc<PersistentDescriptorSet>) {
        let set_layouts = self.graphics_pipeline.layout().set_layouts();
        let layout = set_layouts.get(2).unwrap();

        let vertex_set = PersistentDescriptorSet::new(
            &renderer.get_descriptor_set_allocator(),
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, vertex_buffer.clone()),
            ]
        ).unwrap();

        let default_descriptors = renderer.default_lit_descriptors(&params);

        (default_descriptors.0, default_descriptors.1, vertex_set)
    }

    pub fn generate_vertices(
        &self,
        renderer: &mut MeshRenderer,
        vertex_buffer: Subbuffer<[[f32; 4]]>,
        indirect_buffer: Subbuffer<[DrawIndirectCommand]>,
        objects: &Vec<Metaball>,
    ) {
        let compute_descriptors = self.compute_descriptors(renderer, vertex_buffer, indirect_buffer, objects);
        renderer.get_base_mut().commands_mut()
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.compute_pipeline.layout().clone(),
                0,
                compute_descriptors,
            )
            .bind_pipeline_compute(self.compute_pipeline.clone())
            .dispatch([32, 32, 32])
            .unwrap();
    }
}

fn create_graphics_pipeline(renderer: &MeshRenderer) -> Arc<GraphicsPipeline> {
    let base = renderer.get_base();
    let device = base.get_device();

    let albedo_pass = Subpass::from(renderer.get_render_pass().clone(), 0).unwrap();
    let dimensions = base.get_viewport().dimensions;

    let fs = fs::load(device.clone()).unwrap();
    let vs = vs::load(device.clone()).unwrap();

    GraphicsPipeline::start()
        // .vertex_input_state(vulkano::pipeline::graphics::vertex_input::VertexInputState::)
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
            Viewport {
                origin: [0.0, 0.0],
                dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                depth_range: 0.0..1.0,
            },
        ]))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
        .render_pass(albedo_pass)
        .build(base.get_device().clone())
        .unwrap()
}


/// Parses a string, like the ones found in `triangle_counts.txt` and `vertex_edge_indices.txt`,
/// into a buffer of `u32`s.
fn get_u32_buf(
    data: &'static str,
    buffer_allocator: &(impl MemoryAllocator + ?Sized),
    render_base: &RenderBase
) -> Subbuffer<[Padded<u32, 12>]> {
    let values = data
        .split(" ")
        .map(|s| Padded::from(s.parse::<u32>().unwrap()))
        .collect::<Vec<Padded<u32, 12>>>();
    let num_values = values.len();

    Buffer::from_iter(
        buffer_allocator,
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC | BufferUsage::UNIFORM_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            usage: MemoryUsage::Upload,
            ..Default::default()
        },
        values
            .into_iter(),
    )
        .unwrap()
        .into_device_local(num_values as u64, buffer_allocator, &render_base)

}