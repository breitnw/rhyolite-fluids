use crate::UnconfiguredError;
use crate::geometry::MeshObject;
use crate::geometry::dummy::DummyVertex;
use crate::geometry::loader::BasicVertex;
use crate::shaders::{albedo_vert, point_frag, ambient_frag, Shaders, unlit_vert, albedo_frag};
use crate::lighting::{AmbientLight, PointLight};
use crate::camera::Camera;

use vulkano;
use vulkano::buffer::{CpuBufferPool, TypedBufferAccess, CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::SubpassContents;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::image::{AttachmentImage, SwapchainImage, ImageAccess};
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{GenericMemoryAllocator, FreeListAllocator, MemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{ColorBlendState, BlendFactor, AttachmentBlend, BlendOp};
use vulkano::pipeline::graphics::rasterization::{RasterizationState, CullMode};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{ViewportState, Viewport};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::render_pass::{Subpass, RenderPass, Framebuffer, FramebufferCreateInfo};
use vulkano::format::Format;
use winit::event_loop::EventLoop;

use std::sync::Arc;

use super::{Renderer, RenderBase};

// TODO: Store all functions for advancing stages in an impl for this enum

#[derive(Debug, Clone, PartialEq)]
enum RenderStage {
    Stopped,
    Albedo,
    Ambient,
    Point,
    Unlit,
}

pub struct MeshRenderer {
    base: RenderBase,

    render_pass: Arc<RenderPass>,

    buffer_allocator: Arc<GenericMemoryAllocator<Arc<FreeListAllocator>>>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,

    ambient_light_buf: Option<Arc<CpuAccessibleBuffer<ambient_frag::ty::Ambient_Light_Data>>>,
    point_light_buf_pool: CpuBufferPool<point_frag::ty::Point_Light_Data>,
    albedo_buf_pool: CpuBufferPool<albedo_vert::ty::Model_Data>,
    unlit_buf_pool: CpuBufferPool<unlit_vert::ty::Model_Data>,
    vp_buf_pool: CpuBufferPool<albedo_vert::ty::VP_Data>,

    vp_set: Option<Arc<PersistentDescriptorSet>>,

    albedo_pipeline: Arc<GraphicsPipeline>,
    point_light_pipeline: Arc<GraphicsPipeline>,
    ambient_light_pipeline: Arc<GraphicsPipeline>,
    unlit_pipeline: Arc<GraphicsPipeline>,

    dummy_vertices: Arc<CpuAccessibleBuffer<[DummyVertex]>>,

    framebuffers: Vec<Arc<Framebuffer>>,
    attachment_buffers: AttachmentBuffers,

    render_stage: RenderStage,
}

impl MeshRenderer {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let mut base = RenderBase::new(&event_loop);

        let shaders = Shaders::default(&base.device);

        // Declare the render pass, a structure that lets us define how the rendering process should work. Tells the hardware
        // where it can expect to find input and where it can store output
        let render_pass = get_render_pass(&base.device, base.swapchain.image_format());
        let albedo_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

        // Render pipelines
        let albedo_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<BasicVertex>())
            .vertex_shader(shaders.albedo.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(shaders.albedo.frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(albedo_pass)
            .build(base.device.clone())
            .unwrap();

        let point_light_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(shaders.point.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(shaders.point.frag.entry_point("main").unwrap(), ())
            .color_blend_state(ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                AttachmentBlend {
                    color_op: BlendOp::Add,
                    color_source: BlendFactor::One,
                    color_destination: BlendFactor::One,
                    alpha_op: BlendOp::Max, 
                    alpha_source: BlendFactor::One,
                    alpha_destination: BlendFactor::One,
                }
            ))
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(base.device.clone())
            .unwrap();

        let ambient_light_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(shaders.ambient.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(shaders.ambient.frag.entry_point("main").unwrap(), ())
            .color_blend_state(ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                AttachmentBlend {
                    color_op: BlendOp::Add,
                    color_source: BlendFactor::One,
                    color_destination: BlendFactor::One,
                    alpha_op: BlendOp::Max,
                    alpha_source: BlendFactor::One,
                    alpha_destination: BlendFactor::One,
                }
            ))
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(base.device.clone())
            .unwrap();

        let unlit_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<BasicVertex>())
            .vertex_shader(shaders.unlit.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(shaders.unlit.frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(base.device.clone())
            .unwrap();

        // Buffer allocators
        // Generic allocator for framebuffer attachments, descriptor sets, vertex buffers, etc. 
        // TODO: might want to have multiple allocators separated based on function
        let buffer_allocator = Arc::from(GenericMemoryAllocator::<Arc<FreeListAllocator>>::new_default(base.device.clone()));
        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(base.device.clone());

        // Buffers and buffer pools
        let ambient_light_buf = None;
        let point_light_buf_pool = CpuBufferPool::<point_frag::ty::Point_Light_Data>::uniform_buffer(buffer_allocator.clone());
        let albedo_buf_pool = CpuBufferPool::<albedo_vert::ty::Model_Data>::uniform_buffer(buffer_allocator.clone());
        let unlit_buf_pool = CpuBufferPool::<unlit_vert::ty::Model_Data>::uniform_buffer(buffer_allocator.clone());
        let vp_buf_pool = CpuBufferPool::<albedo_vert::ty::VP_Data>::uniform_buffer(buffer_allocator.clone());

        // Create a dummy vertex buffer used for full-screen shaders
        let dummy_vertices = CpuAccessibleBuffer::from_iter(
            &buffer_allocator, 
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            }, 
            false,
            DummyVertex::list().into_iter(),
        ).unwrap();

        // Includes framebuffers and other attachments that aren't stored
        let (framebuffers, attachment_buffers) = window_size_dependent_setup(
            &buffer_allocator,
            &base.images, 
            render_pass.clone(), 
            &mut base.viewport
        );

        Self { 
            base, 

            buffer_allocator,
            descriptor_set_allocator,

            ambient_light_buf, 
            point_light_buf_pool, 
            albedo_buf_pool, 
            unlit_buf_pool, 
            vp_buf_pool, 

            vp_set: None, 

            albedo_pipeline,
            point_light_pipeline, 
            ambient_light_pipeline, 
            unlit_pipeline, 

            dummy_vertices, 

            render_pass,
            framebuffers,
            attachment_buffers,

            render_stage: RenderStage::Stopped,
        }
    }


    /// Starts the rendering process for the current frame
    pub fn start(&mut self, camera: &mut Camera) {
        let base = &self.base;

        if !camera.is_configured() {
            camera.configure(&base.window);
        }

        let vp_layout = self.albedo_pipeline.layout().set_layouts().get(0).unwrap().clone();
        let vp_subbuffer = camera.get_vp_subbuffer(&self.vp_buf_pool).unwrap();
        self.vp_set = Some(PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            vp_layout,
            [
                WriteDescriptorSet::buffer(0, vp_subbuffer),
            ]
        ).unwrap());

        if self.base.should_recreate_swapchain {
            camera.configure(&base.window);
            self.recreate_swapchain_and_buffers();
        }

        self.base.start(&mut self.framebuffers);
    }

    /// Finishes the rendering process and draws to the screen
    /// # Panics
    /// Panics if not called after a `draw_object_unlit()` call or a `draw_point()` call
    pub fn finish(&mut self) {
        if self.base.render_error { return; }
        self.update_render_stage(RenderStage::Stopped);
        self.base.finish();
    }


    /// Draws an object that will later be lit
    /// # Panics
    /// Panics if not called after a `start()` call or another `draw_object()` call 
    pub fn draw_object(&mut self, object: &mut MeshObject) -> Result<(), UnconfiguredError> {
        
        if self.base.render_error  { return Ok(()); }
        self.update_render_stage(RenderStage::Albedo);

        let albedo_subbuffer = {
            let (model_mat, normal_mat) = object.transform.get_rendering_matrices();
            let uniform_data = albedo_vert::ty::Model_Data {
                model: model_mat.into(),
                normals: normal_mat.into(),
            };
            self.albedo_buf_pool.from_data(uniform_data).unwrap()
        };
        
        // TODO: Do this with textures instead!!!!!!!!! Not a CpuAccessibleBuffer!!!!!!!!!
        // or at least store the buffer instead of recreating it every frame.....
        let (intensity, shininess) = object.get_specular();
        let specular_buffer = CpuAccessibleBuffer::from_data(
            &self.buffer_allocator, 
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            }, 
            false, 
            albedo_frag::ty::Specular_Data {
                intensity,
                shininess,
            },
        ).unwrap();

        let albedo_layout = self.albedo_pipeline.layout().set_layouts().get(1).unwrap().clone();
        let albedo_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            albedo_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, albedo_subbuffer),
                WriteDescriptorSet::buffer(1, specular_buffer),
            ]
        ).unwrap();

        // Add albedo-related commands to the command buffer
        self.base.commands_mut()
            .bind_pipeline_graphics(self.albedo_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.albedo_pipeline.layout().clone(), 
                0,
                (self.vp_set.as_ref().unwrap().clone(), albedo_set.clone())
            )
            // TODO: possible to bind multiple vertex buffers at once?
            .bind_vertex_buffers(0, object.get_vertex_buffer()?.clone())
            .draw(object.get_vertex_buffer()?.len() as u32, 1, 0, 0)
            .unwrap();
        
        Ok(())
    }
    
    /// Sets the ambient light to use for rendering
    pub fn set_ambient(&mut self, light: AmbientLight) {
        self.ambient_light_buf = Some(CpuAccessibleBuffer::from_data(
            &self.buffer_allocator, 
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            }, 
            false, 
            ambient_frag::ty::Ambient_Light_Data {
                color: light.color.into(),
                intensity: light.intensity.into(),
            },
        ).unwrap())
    }

    /// Draws an ambient light, which adds global illumination to the entire scene
    /// # Panics
    /// Panics if not called after a `draw_object()` call
    pub fn draw_ambient_light(&mut self) {
        if self.base.render_error  { return; }
        self.update_render_stage(RenderStage::Ambient);

        if self.ambient_light_buf.is_none() { 
            self.base.commands_mut()
                .next_subpass(SubpassContents::Inline)
                .unwrap();
            return; 
        }

        let ambient_layout = self.ambient_light_pipeline.layout().set_layouts().get(0).unwrap();
        let ambient_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            ambient_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.attachment_buffers.albedo_buffer.clone()),
                WriteDescriptorSet::buffer(1, self.ambient_light_buf.as_mut().unwrap().clone()),
            ],
        ).unwrap();

        // Add ambient light commands to the command buffer
        self.base.commands_mut()
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(self.ambient_light_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.ambient_light_pipeline.layout().clone(), 
                0,
                ambient_set.clone(),
            )
            .bind_vertex_buffers(0, self.dummy_vertices.clone())
            .draw(self.dummy_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }

    /// Draws a point light with a specified color and position
    /// # Panics
    /// Panics if not called after a `draw_ambient()` call or `another draw_point()` call
    pub fn draw_point_light(&mut self, camera: &mut Camera, point_light: &mut PointLight) {
        if self.base.render_error { return; }
        self.update_render_stage(RenderStage::Point);

        let point_subbuffer = point_light.get_buffer(&self.point_light_buf_pool);

        let point_layout = self.point_light_pipeline.layout().set_layouts().get(0).unwrap().clone();
        let point_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            point_layout.clone(),
            [ 
                WriteDescriptorSet::image_view(0, self.attachment_buffers.albedo_buffer.clone()),
                WriteDescriptorSet::image_view(1, self.attachment_buffers.normal_buffer.clone()),
                WriteDescriptorSet::image_view(2, self.attachment_buffers.frag_pos_buffer.clone()),
                WriteDescriptorSet::image_view(3, self.attachment_buffers.specular_buffer.clone()),
                WriteDescriptorSet::buffer(4, point_subbuffer),
            ],
        ).unwrap();

        self.base.commands_mut()
            .bind_pipeline_graphics(self.point_light_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.point_light_pipeline.layout().clone(),
                0,
                (self.vp_set.as_ref().unwrap().clone(), point_set),
            )
            .bind_vertex_buffers(0, self.dummy_vertices.clone())
            .draw(self.dummy_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }

    /// Draws an object with an unlit shader by rendering it after shadows are drawn
    /// # Panics
    /// Panics if not called after a `draw_point()` call or another `draw_object_unlit()` call
    pub fn draw_object_unlit(&mut self, object: &mut MeshObject) -> Result<(), UnconfiguredError> {
        if self.base.render_error { return Ok(()); }
        self.update_render_stage(RenderStage::Unlit);

        let unlit_subbuffer = {
            let (model_mat, normal_mat) = object.transform.get_rendering_matrices();
            let uniform_data = albedo_vert::ty::Model_Data {
                model: model_mat.into(),
                normals: normal_mat.into(),
            };
            self.albedo_buf_pool.from_data(uniform_data).unwrap()
        };
        let unlit_layout = self.unlit_pipeline.layout().set_layouts().get(1).unwrap().clone();
        let unlit_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            unlit_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, unlit_subbuffer)
            ]
        ).unwrap();

        // Add commands to the command buffer
        self.base.commands_mut()
            .bind_pipeline_graphics(self.unlit_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.unlit_pipeline.layout().clone(), 
                0,
                (self.vp_set.as_ref().unwrap().clone(), unlit_set.clone())
            )
            // TODO: possible to bind multiple vertex buffers at once?
            .bind_vertex_buffers(0, object.get_vertex_buffer()?.clone())
            .draw(object.get_vertex_buffer()?.len() as u32, 1, 0, 0)
            .unwrap();
        Ok(())
    }

    /// Updates the aspect ratio of the camera. Should be called when the window is resized
    pub fn update_aspect_ratio(&mut self, camera: &mut Camera) {
        camera.configure(&self.base.window);
    }

    /// Sets up necessary buffers and attaches them to the object
    pub fn configure_object(&self, object: &mut MeshObject) {
        object.configure(&self.buffer_allocator)
    }

    fn get_render_stage(&self) -> &RenderStage { &self.render_stage }
    fn set_render_stage(&mut self, new_stage: RenderStage) {
        self.render_stage = new_stage;
    }
    fn update_render_stage(&mut self, new_stage: RenderStage) {
        let mut out_of_order = false;
        match new_stage {
            RenderStage::Albedo => match self.render_stage {
                RenderStage::Stopped => {
                    self.render_stage = RenderStage::Albedo;
                }
                RenderStage::Albedo => (),
                _ => out_of_order = true
            }
            RenderStage::Ambient => match self.render_stage {
                RenderStage::Albedo => {
                    self.render_stage = RenderStage::Ambient;
                },
                _ => out_of_order = true
            }
            RenderStage::Point => match self.render_stage {
                RenderStage::Ambient => {
                    self.render_stage = RenderStage::Point;
                }
                RenderStage::Point => (),
                _ => out_of_order = true
            }
            RenderStage::Unlit => match self.render_stage {
                RenderStage::Ambient | RenderStage::Point => {
                    self.render_stage = RenderStage::Unlit;
                }
                RenderStage::Unlit => (),
                _ => out_of_order = true
            }
            RenderStage::Stopped => match self.render_stage {
                RenderStage::Point | RenderStage::Unlit => {
                    self.render_stage = RenderStage::Stopped;
                },
                _ => out_of_order = true
            },
        }
        if out_of_order {
            panic!("can't enter {:?} stage after {:?} stage, rendering stopped", new_stage, self.render_stage)
        }
    }
}


impl Renderer for MeshRenderer {
    type Object = MeshObject;

    fn recreate_swapchain_and_buffers(&mut self) {
        self.base.recreate_swapchain();
        // TODO: use a different allocator?
        (self.framebuffers, self.attachment_buffers) = window_size_dependent_setup(
            &self.buffer_allocator, 
            &self.base.images, 
            self.render_pass.clone(), 
            &mut self.base.viewport
        );
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
) -> (Vec<Arc<Framebuffer>>, AttachmentBuffers) {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(allocator, dimensions, Format::D16_UNORM).unwrap()
    ).unwrap();
    let albedo_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator, 
            dimensions, 
            Format::A2B10G10R10_UNORM_PACK32,
        ).unwrap()
    ).unwrap();
    let normal_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator, 
            dimensions, 
            Format::R16G16B16A16_SFLOAT,
        ).unwrap()
    ).unwrap();
    let frag_pos_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator, 
            dimensions, 
            Format::R16G16B16A16_SFLOAT
        ).unwrap()
    ).unwrap();
    let specular_buffer = ImageView::new_default(
        AttachmentImage::transient_input_attachment(
            allocator, 
            dimensions, 
            Format::R16G16_SFLOAT,
        ).unwrap()
    ).unwrap();
    
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
                        depth_buffer.clone()
                    ],
                    ..Default::default()
                }
            ).unwrap()
        }).collect::<Vec<_>>();

    let attachment_buffers = AttachmentBuffers {
        albedo_buffer: albedo_buffer.clone(),
        normal_buffer: normal_buffer.clone(),
        frag_pos_buffer: frag_pos_buffer.clone(),
        specular_buffer: specular_buffer.clone(),
    };
    (framebuffers, attachment_buffers)
}

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