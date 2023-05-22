use crate::camera::Camera;
use crate::geometry::dummy::DummyVertex;
use crate::geometry::mesh::loader::BasicVertex;
use crate::geometry::mesh::{MeshObject, MeshObjectBuilder};
use crate::lighting::{AmbientLight, PointLight};
use crate::renderer::staging::{IntoPersistentUniform, UniformSrc};
use crate::shaders::{albedo_frag, Shaders};

use vulkano;
use vulkano::buffer::{BufferUsage, Subbuffer};
use vulkano::command_buffer::{DrawIndirectCommand, SubpassContents};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageAccess, SwapchainImage};
use vulkano::memory::allocator::{MemoryAllocator, MemoryUsage, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, BlendOp, ColorBlendState,
};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use winit::event_loop::EventLoop;

use std::sync::Arc;
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use crate::transform::Transform;

use super::{RenderBase, Renderer};

/// An enum representing the sequential stages of rendering necessary for construction of the
/// command buffer. Since Rhyolite's Mesh engine uses deferred rendering, they must be added
/// in the following order:
/// 1. Albedo
/// 2. Ambient
/// 3. Point (optional)
/// 4. Unlit (optional)
#[derive(Debug, Clone, PartialEq)]
enum RenderStage {
    Stopped,
    Albedo,
    Ambient,
    Point,
    Unlit,
}

impl RenderStage {
    /// Advances this `RenderStage`'s value to match that of new_stage.
    /// # Panics
    /// Since advancement between `RenderStage`s is meant to be manually implemented rather than
    /// determined programmatically, the purpose of this function is to panic if the stages are
    /// called out of order. The function will panic in these circumstances:
    /// 1. Trying to enter `RenderStage::Albedo` when the current stage is something other than
    /// `RenderStage::Stopped` or `RenderStage::Albedo`
    /// 2. Trying to enter `RenderStage::Ambient` when the current stage is something other than
    /// `RenderStage::Albedo`
    /// 3. Trying to enter `RenderStage::Point` when the current stage is something other than
    /// `RenderStage::Ambient` or `RenderStage::Point`
    /// 4. Trying to enter `RenderStage::Unlit` when the current stage is something other than
    /// `RenderStage::Ambient`, `RenderStage::Point`, or `RenderStage::Unlit`
    /// 5. Trying to enter `RenderStage::Stopped` (usually by calling the renderer's `finish()`
    /// function) when the current stage is something other than `RenderStage::Ambient`,
    /// `RenderStage::Point`, or `RenderStage::Unlit`
    fn update(&mut self, new_stage: RenderStage) {
        let mut out_of_order = false;
        match new_stage {
            RenderStage::Albedo => match self {
                RenderStage::Stopped => {
                    *self = RenderStage::Albedo;
                }
                RenderStage::Albedo => (),
                _ => out_of_order = true,
            },
            RenderStage::Ambient => match self {
                RenderStage::Albedo => {
                    *self = RenderStage::Ambient;
                }
                _ => out_of_order = true,
            },
            RenderStage::Point => match self {
                RenderStage::Ambient => {
                    *self = RenderStage::Point;
                }
                RenderStage::Point => (),
                _ => out_of_order = true,
            },
            RenderStage::Unlit => match self {
                RenderStage::Ambient | RenderStage::Point => {
                    *self = RenderStage::Unlit;
                }
                RenderStage::Unlit => (),
                _ => out_of_order = true,
            },
            RenderStage::Stopped => match self {
                RenderStage::Ambient | RenderStage::Point | RenderStage::Unlit => {
                    *self = RenderStage::Stopped;
                }
                _ => out_of_order = true,
            },
        }
        if out_of_order {
            panic!(
                "can't enter {:?} stage after {:?} stage, rendering stopped",
                new_stage, self
            )
        }
    }
}

pub struct MeshRenderer {
    base: RenderBase,

    render_pass: Arc<RenderPass>,

    buffer_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    subbuffer_allocator: SubbufferAllocator,

    vp_set: Option<Arc<PersistentDescriptorSet>>,

    dummy_vertex_buf: Subbuffer<[DummyVertex]>,

    pipelines: Pipelines,
    framebuffers: Vec<Arc<Framebuffer>>,
    attachment_buffers: AttachmentBuffers,

    render_stage: RenderStage,
}

impl MeshRenderer {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let mut base = RenderBase::new(&event_loop);

        // Declare the render pass, a structure that lets us define how the rendering process should work. Tells the hardware
        // where it can expect to find input and where it can store output
        let render_pass = get_render_pass(&base.device, base.swapchain.image_format());
        // let pipelines = Pipelines::new(&render_pass, &device);

        // Buffer allocators
        // Generic allocator for framebuffer attachments, descriptor sets, vertex buffers, etc.
        // TODO: might want to have multiple allocators separated based on function
        let buffer_allocator = Arc::from(StandardMemoryAllocator::new_default(base.device.clone()));
        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = Arc::from(StandardDescriptorSetAllocator::new(base.device.clone()));

        // A buffer pool for all data used in rendering, including ambient lights, point lights, albedo, unlit, and view/projection matrices
        let subbuffer_allocator = SubbufferAllocator::new(
            buffer_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                arena_size: 512, // TODO: FIND THE ACTUAL VALUE
                buffer_usage: BufferUsage::UNIFORM_BUFFER,
                memory_usage: MemoryUsage::Upload,
                ..Default::default()
            },
        );

        // Create a dummy vertex buffer used for full-screen shaders
        let dummy_vertex_buf = DummyVertex::buf(&buffer_allocator, &base);

        // Includes framebuffers and other attachments that aren't stored
        let (framebuffers, attachment_buffers, pipelines) = window_size_dependent_setup(
            &buffer_allocator,
            &base.images,
            render_pass.clone(),
            &mut base.viewport,
            &base.device,
        );

        Self {
            base,

            render_pass,

            buffer_allocator,
            descriptor_set_allocator,
            subbuffer_allocator,

            vp_set: None,

            dummy_vertex_buf,

            pipelines,
            framebuffers,
            attachment_buffers,

            render_stage: RenderStage::Stopped,
        }
    }

    /// Starts the rendering process for the current frame
    pub fn start_render_pass(&mut self, camera: &mut Camera) {
        if !camera.is_configured() {
            camera.configure(self.get_window_size());
        }

        let vp_layout = self
            .pipelines
            .albedo
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone();
        let vp_subbuffer = camera.get_vp_subbuffer(&self.subbuffer_allocator).unwrap();
        self.vp_set = Some(
            PersistentDescriptorSet::new(
                &self.descriptor_set_allocator,
                vp_layout,
                [WriteDescriptorSet::buffer(0, vp_subbuffer)],
            )
            .unwrap(),
        );

        if self.base.should_recreate_swapchain {
            camera.configure(self.get_window_size());
            self.recreate_all_size_dependent();
        }

        self.base.start(&mut self.framebuffers);
    }

    /// Finishes the rendering process and draws to the screen
    /// # Panics
    /// Panics if not called after a `draw_object_unlit()` call or a `draw_point()` call
    pub fn finish(&mut self) {
        if self.base.render_error {
            return;
        }
        self.render_stage.update(RenderStage::Stopped);
        self.base.finish();
    }

    /// Adds a mesh (vertex buffer) to the command buffer without drawing it. This is done so that
    /// both `draw()` and `draw_indirect()` functions may be used depending on the use case.
    fn add_object(&mut self, object: &MeshObject) {
        self.render_stage.update(RenderStage::Albedo);

        let albedo_subbuffer = self.subbuffer_allocator.allocate_sized().unwrap();
        *albedo_subbuffer.write().unwrap() = object.get_raw();

        // TODO: Do this with textures instead!!!!!!!!! Not a subbuffer!!!!!!!!!
        // or at least store the buffer instead of recreating it every frame.....
        let (intensity, shininess) = object.get_specular();

        let specular_subbuffer = self.subbuffer_allocator.allocate_sized().unwrap();
        *specular_subbuffer.write().unwrap() = albedo_frag::USpecularData {
            intensity,
            shininess,
        };

        let albedo_layout = self
            .pipelines
            .albedo
            .layout()
            .set_layouts()
            .get(1)
            .unwrap()
            .clone();
        let albedo_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            albedo_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, albedo_subbuffer),
                WriteDescriptorSet::buffer(1, specular_subbuffer),
            ],
        )
            .unwrap();

        // Add albedo-related commands to the command buffer
        self.base
            .commands_mut()
            .bind_pipeline_graphics(self.pipelines.albedo.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipelines.albedo.layout().clone(),
                0,
                (self.vp_set.as_ref().unwrap().clone(), albedo_set.clone()),
            )
            // TODO: possible to bind multiple vertex buffers at once?
            .bind_vertex_buffers(0, object.get_vertex_buffer().clone());
    }

    /// Draws an object that will later be lit
    /// # Panics
    /// Panics if not called after a `start()` call or another `draw_object()` call
    pub fn draw_object(&mut self, object: &MeshObject) {
        if self.base.render_error {
            return;
        }
        self.add_object(object);
        self.base.commands_mut()
            .draw(object.get_vertex_buffer().len() as u32, 1, 0, 0)
            .unwrap();
    }

    /// Draws an in indirect object, usually with a vertex buffer generated by a compute shader,
    /// that will later be lit
    /// # Panics
    /// Panics if not called after a `start()` call or another `draw_object()` call
    pub fn draw_object_indirect(
        &mut self,
        object: &MeshObject,
        indirect_buffer: Subbuffer<[DrawIndirectCommand]>,
    ) {
        if self.base.render_error {
            return;
        }
        self.add_object(object);
        self.base.commands_mut()
            .draw_indirect(indirect_buffer)
            .unwrap();
    }

    // TODO: Make MeshObject generic and use that instead
    /// Draws an object based on a custom vertex buffer and graphics pipeline. Lighting data will
    /// still be added later. **THIS FUNCTION SUCKS RIGHT NOW DON'T USE IT PLEASEEEEEE**
    /// # Panics
    /// Panics if not called after a `start()` call or another `draw_object()` call
    pub fn draw_object_pipeline<T: Vertex>(&mut self, pipeline: &Arc<GraphicsPipeline>, vertex_buffer: Subbuffer<[T]>, transform: &Transform) {
        self.render_stage.update(RenderStage::Albedo);

        let albedo_subbuffer = self.subbuffer_allocator.allocate_sized().unwrap();
        *albedo_subbuffer.write().unwrap() = {
            let (model_mat, normal_mat) = transform.get_matrices();
            crate::shaders::albedo_vert::UModelData {
                model: model_mat.into(),
                normals: normal_mat.into(),
            }
        };

        let (intensity, shininess) = (1.0, 64.0);

        let specular_subbuffer = self.subbuffer_allocator.allocate_sized().unwrap();
        *specular_subbuffer.write().unwrap() = albedo_frag::USpecularData {
            intensity,
            shininess,
        };

        let albedo_layout = pipeline
            .layout()
            .set_layouts()
            .get(1)
            .unwrap()
            .clone();
        let albedo_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            albedo_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, albedo_subbuffer),
                WriteDescriptorSet::buffer(1, specular_subbuffer),
            ],
        ).unwrap();

        // Add albedo-related commands to the command buffer
        self.base
            .commands_mut()
            .bind_pipeline_graphics(pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipelines.albedo.layout().clone(),
                0,
                (self.vp_set.as_ref().unwrap().clone(), albedo_set.clone()),
            )
            // TODO: possible to bind multiple vertex buffers at once?
            .bind_vertex_buffers(0, vertex_buffer);
    }

    /// Draws an ambient light, which adds global illumination to the entire scene
    /// # Panics
    /// Panics if not called after a `draw_object()` call
    pub fn draw_ambient_light(&mut self, light: &mut AmbientLight) {
        if self.base.render_error {
            return;
        }
        self.render_stage.update(RenderStage::Ambient);

        let ambient_layout = self
            .pipelines
            .ambient
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();

        let ambient_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            ambient_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.attachment_buffers.albedo_buffer.clone()),
                WriteDescriptorSet::buffer(1, light.get_buffer(&self.buffer_allocator, &self.base)),
            ],
        )
        .unwrap();

        // Add ambient light commands to the command buffer
        self.base
            .commands_mut()
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(self.pipelines.ambient.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipelines.ambient.layout().clone(),
                0,
                ambient_set.clone(),
            )
            .bind_vertex_buffers(0, self.dummy_vertex_buf.clone())
            .draw(self.dummy_vertex_buf.len() as u32, 1, 0, 0)
            .unwrap();
    }

    /// Draws a point light with a specified color and position
    /// # Panics
    /// Panics if not called after a `draw_ambient()` call or `another draw_point()` call
    pub fn draw_point_light(&mut self, light: &mut PointLight) {
        if self.base.render_error {
            return;
        }
        self.render_stage.update(RenderStage::Point);

        let point_layout = self
            .pipelines
            .point
            .layout()
            .set_layouts()
            .get(1)
            .unwrap()
            .clone();

        let point_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            point_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.attachment_buffers.albedo_buffer.clone()),
                WriteDescriptorSet::image_view(1, self.attachment_buffers.normal_buffer.clone()),
                WriteDescriptorSet::image_view(2, self.attachment_buffers.frag_pos_buffer.clone()),
                WriteDescriptorSet::image_view(3, self.attachment_buffers.specular_buffer.clone()),
                WriteDescriptorSet::buffer(4, light.get_buffer(&self.buffer_allocator, &self.base)),
            ],
        )
        .unwrap();

        self.base
            .commands_mut()
            .bind_pipeline_graphics(self.pipelines.point.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipelines.point.layout().clone(),
                0,
                (self.vp_set.as_ref().unwrap().clone(), point_set),
            )
            .bind_vertex_buffers(0, self.dummy_vertex_buf.clone())
            .draw(self.dummy_vertex_buf.len() as u32, 1, 0, 0)
            .unwrap();
    }

    /// Draws an object with an unlit shader by rendering it after shadows are drawn
    /// # Panics
    /// Panics if not called after a `draw_point()` call or another `draw_object_unlit()` call
    pub fn draw_object_unlit(&mut self, object: &mut MeshObject) {
        if self.base.render_error {
            return;
        }
        self.render_stage.update(RenderStage::Unlit);

        let unlit_subbuffer = self.subbuffer_allocator.allocate_sized().unwrap();
        *unlit_subbuffer.write().unwrap() = object.get_raw();

        let unlit_layout = self
            .pipelines
            .unlit
            .layout()
            .set_layouts()
            .get(1)
            .unwrap()
            .clone();
        let unlit_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            unlit_layout.clone(),
            [WriteDescriptorSet::buffer(0, unlit_subbuffer)],
        )
        .unwrap();

        // Add commands to the command buffer
        self.base
            .commands_mut()
            .bind_pipeline_graphics(self.pipelines.unlit.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipelines.unlit.layout().clone(),
                0,
                (self.vp_set.as_ref().unwrap().clone(), unlit_set.clone()),
            )
            // TODO: possible to bind multiple vertex buffers at once?
            .bind_vertex_buffers(0, object.get_vertex_buffer().clone())
            .draw(object.get_vertex_buffer().len() as u32, 1, 0, 0)
            .unwrap();
    }

    fn get_render_stage(&self) -> &RenderStage {
        &self.render_stage
    }
    fn set_render_stage(&mut self, new_stage: RenderStage) {
        self.render_stage = new_stage;
    }

    pub fn get_buffer_allocator(&self) -> Arc<StandardMemoryAllocator> {
        self.buffer_allocator.clone()
    }
    pub fn get_descriptor_set_allocator(&self) -> Arc<StandardDescriptorSetAllocator> {
        self.descriptor_set_allocator.clone()
    }
    pub fn get_render_pass(&self) -> Arc<RenderPass> {
        self.render_pass.clone()
    }

    pub fn get_base_mut(&mut self) -> &mut RenderBase {
        &mut self.base
    }
}

impl Renderer for MeshRenderer {
    /// Recreates all of the structures dependent on the window size, including the framebuffers,
    /// attachment buffers, swapchain, and pipelines
    fn recreate_all_size_dependent(&mut self) {
        self.base.recreate_swapchain();
        // TODO: use a different allocator?
        let (framebuffers, attachment_buffers, pipelines) = window_size_dependent_setup(
            &self.buffer_allocator,
            &self.base.images,
            self.render_pass.clone(),
            &mut self.base.viewport,
            &self.base.device,
        );
        self.framebuffers = framebuffers;
        self.attachment_buffers = attachment_buffers;
        self.pipelines = pipelines;
    }

    fn get_base(&self) -> &RenderBase {
        &self.base
    }
}

pub(crate) struct AttachmentBuffers {
    pub albedo_buffer: Arc<ImageView<AttachmentImage>>,
    pub normal_buffer: Arc<ImageView<AttachmentImage>>,
    pub frag_pos_buffer: Arc<ImageView<AttachmentImage>>,
    pub specular_buffer: Arc<ImageView<AttachmentImage>>,
}

/// Sets up the framebuffers based on the size of the viewport.
fn window_size_dependent_setup(
    allocator: &(impl MemoryAllocator + ?Sized),
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
    device: &Arc<Device>,
) -> (Vec<Arc<Framebuffer>>, AttachmentBuffers, Pipelines) {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(allocator, dimensions, Format::D16_UNORM).unwrap(),
    )
    .unwrap();
    let albedo_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator,
            dimensions,
            Format::A2B10G10R10_UNORM_PACK32,
        )
        .unwrap(),
    )
    .unwrap();
    let normal_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator,
            dimensions,
            Format::R16G16B16A16_SFLOAT,
        )
        .unwrap(),
    )
    .unwrap();
    let frag_pos_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator,
            dimensions,
            Format::R16G16B16A16_SFLOAT,
        )
        .unwrap(),
    )
    .unwrap();
    let specular_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(allocator, dimensions, Format::R16G16_SFLOAT)
            .unwrap(),
    )
    .unwrap();

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        view,
                        albedo_buffer.clone(),
                        normal_buffer.clone(),
                        frag_pos_buffer.clone(),
                        specular_buffer.clone(),
                        depth_buffer.clone(),
                    ],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    let attachment_buffers = AttachmentBuffers {
        albedo_buffer: albedo_buffer.clone(),
        normal_buffer: normal_buffer.clone(),
        frag_pos_buffer: frag_pos_buffer.clone(),
        specular_buffer: specular_buffer.clone(),
    };

    let pipelines = Pipelines::new(&render_pass, dimensions, device);

    (framebuffers, attachment_buffers, pipelines)
}

struct Pipelines {
    albedo: Arc<GraphicsPipeline>,
    point: Arc<GraphicsPipeline>,
    ambient: Arc<GraphicsPipeline>,
    unlit: Arc<GraphicsPipeline>,
}

impl Pipelines {
    pub fn new(render_pass: &Arc<RenderPass>, dimensions: [u32; 2], device: &Arc<Device>) -> Self {
        let shaders = Shaders::mesh_default(device);

        // Declare the render pass, a structure that lets us define how the rendering process should work. Tells the hardware
        // where it can expect to find input and where it can store output
        let albedo_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

        // Render pipelines
        let albedo = GraphicsPipeline::start()
            .vertex_input_state(BasicVertex::per_vertex())
            .vertex_shader(shaders.albedo.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
                Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0..1.0,
                },
            ]))
            .fragment_shader(shaders.albedo.frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(albedo_pass)
            .build(device.clone())
            .unwrap();

        let point = GraphicsPipeline::start()
            .vertex_input_state(DummyVertex::per_vertex())
            .vertex_shader(shaders.point.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
                Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0..1.0,
                },
            ]))
            .fragment_shader(shaders.point.frag.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Max,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One,
                    },
                ),
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let ambient = GraphicsPipeline::start()
            .vertex_input_state(DummyVertex::per_vertex())
            .vertex_shader(shaders.ambient.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
                Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0..1.0,
                },
            ]))
            .fragment_shader(shaders.ambient.frag.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Max,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One,
                    },
                ),
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let unlit = GraphicsPipeline::start()
            .vertex_input_state(BasicVertex::per_vertex())
            .vertex_shader(shaders.unlit.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
                Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0..1.0,
                },
            ]))
            .fragment_shader(shaders.unlit.frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        Self {
            albedo,
            point,
            ambient,
            unlit,
        }
    }
}

/// Gets the render pass to use with the Mesh renderer. In Vulkan, a render pass is the set of
/// attachments, the way they are used, and the rendering work that is performed using them.
fn get_render_pass(device: &Arc<Device>, final_format: Format) -> Arc<RenderPass> {
    vulkano::ordered_passes_renderpass!(
        device.clone(),
        attachments: {
            final_color: {
                load: Clear,
                store: Store,
                format: final_format,
                samples: 1,
            },
            albedo: {
                load: Clear,
                store: DontCare,
                format: Format::A2B10G10R10_UNORM_PACK32,
                samples: 1,
            },
            normals: {
                load: Clear,
                store: DontCare,
                format: Format::R16G16B16A16_SFLOAT,
                samples: 1,
            },
            frag_pos: {
                load: Clear,
                store: DontCare,
                format: Format::R16G16B16A16_SFLOAT,
                samples: 1,
            },
            // TODO: textures would typically be used for specular instead of renderpass attachments
            specular: {
                load: Clear,
                store: DontCare,
                format: Format::R16G16_SFLOAT,
                samples: 1,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D16_UNORM,
                samples: 1,
            }
        },
        passes: [
            {
                color: [albedo, normals, frag_pos, specular],
                depth_stencil: {depth},
                input: []
            },
            {
                color: [final_color],
                depth_stencil: {depth},
                input: [albedo, normals, frag_pos, specular]
            }
        ]
    )
    .unwrap()
}
