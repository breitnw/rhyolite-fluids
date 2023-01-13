use crate::UnconfiguredError;
use crate::geometry::MeshObject;
use crate::geometry::dummy::DummyVertex;
use crate::geometry::loader::BasicVertex;
use crate::shaders::{albedo_vert, point_frag, ambient_frag, Shaders, unlit_vert, albedo_frag};
use crate::lighting::{AmbientLight, PointLight};
use crate::camera::Camera;

use vulkano;
use vulkano::buffer::{CpuBufferPool, TypedBufferAccess, CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, PrimaryAutoCommandBuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo, StandardCommandBufferAlloc};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::{Device, Queue};
use vulkano::instance::{Instance};
use vulkano::memory::allocator::{GenericMemoryAllocator, FreeListAllocator};
use vulkano::pipeline::graphics::color_blend::{ColorBlendState, BlendFactor, AttachmentBlend, BlendOp};
use vulkano::pipeline::graphics::rasterization::{RasterizationState, CullMode};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::render_pass::{Subpass, RenderPass, Framebuffer};
use vulkano::swapchain::{
    self, AcquireError, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo, Surface, Swapchain, SwapchainAcquireFuture,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::format::ClearValue;

use vulkano_win::VkSurfaceBuild;

use winit::dpi::LogicalSize;
use winit::event_loop::{EventLoop};
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;

use super::{AttachmentBuffers, Renderer};

#[derive(Debug, Clone, PartialEq)]

enum RenderStage {
    Stopped,
    Albedo,
    Ambient,
    Point,
    Unlit,
    Error,
}
// TODO: Store all functions for advancing stages in an impl for this enum

pub struct MeshRenderer {
    render_pass: Arc<RenderPass>,

    ambient_light_buf: Option<Arc<CpuAccessibleBuffer<ambient_frag::ty::Ambient_Light_Data>>>,
    point_light_buf_pool: CpuBufferPool<point_frag::ty::Point_Light_Data>,
    albedo_buf_pool: CpuBufferPool<albedo_vert::ty::Model_Data>,
    unlit_buf_pool: CpuBufferPool<unlit_vert::ty::Model_Data>,
    vp_buf_pool: CpuBufferPool<albedo_vert::ty::VP_Data>,
    camera_pos_buf_pool: CpuBufferPool<point_frag::ty::Camera_Data>,

    vp_set: Option<Arc<PersistentDescriptorSet>>,

    albedo_pipeline: Arc<GraphicsPipeline>,
    point_light_pipeline: Arc<GraphicsPipeline>,
    ambient_light_pipeline: Arc<GraphicsPipeline>,
    unlit_pipeline: Arc<GraphicsPipeline>,

    dummy_vertices: Arc<CpuAccessibleBuffer<[DummyVertex]>>,
    render_stage: RenderStage,
}

impl MeshRenderer {
    fn new(device: &Arc<Device>, swapchain: Arc<Swapchain>) -> Self {
        let shaders = Shaders::default(&device);

        // Declare the render pass, a structure that lets us define how the rendering process should work. Tells the hardware
        // where it can expect to find input and where it can store output
        let render_pass = super::get_render_pass(&device, swapchain.image_format());
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
            .build(device.clone())
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
            .build(device.clone())
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
            .build(device.clone())
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
            .build(device.clone())
            .unwrap();

        // Generic allocator for framebuffer attachments, descriptor sets, vertex buffers, etc. 
        // TODO: might want to have multiple allocators separated based on function
        let buffer_allocator = Arc::from(GenericMemoryAllocator::<Arc<FreeListAllocator>>::new_default(device.clone()));
        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
        let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), StandardCommandBufferAllocatorCreateInfo::default());

        // Buffers and buffer pools
        let ambient_light_buf = None;
        let point_light_buf_pool = CpuBufferPool::<point_frag::ty::Point_Light_Data>::uniform_buffer(buffer_allocator.clone());
        let albedo_buf_pool = CpuBufferPool::<albedo_vert::ty::Model_Data>::uniform_buffer(buffer_allocator.clone());
        let unlit_buf_pool = CpuBufferPool::<unlit_vert::ty::Model_Data>::uniform_buffer(buffer_allocator.clone());
        let vp_buf_pool = CpuBufferPool::<albedo_vert::ty::VP_Data>::uniform_buffer(buffer_allocator.clone());
        let camera_pos_buf_pool = CpuBufferPool::<point_frag::ty::Camera_Data>::uniform_buffer(buffer_allocator.clone());

        let render_stage = RenderStage::Stopped;

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

        Self { 
            ambient_light_buf, 
            point_light_buf_pool, 
            albedo_buf_pool, 
            unlit_buf_pool, 
            vp_buf_pool, 
            camera_pos_buf_pool, 
            vp_set: None, 
            albedo_pipeline,
            point_light_pipeline, 
            ambient_light_pipeline, 
            unlit_pipeline, 
            dummy_vertices, 
            render_stage,
            render_pass,
        }
    }

    /// Starts the rendering process for the current frame
    pub fn start(&mut self, camera: &mut Camera) {
        if !camera.is_configured() {
            camera.configure(&self.window);
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

        self.previous_frame_end.as_mut()
            .expect("previous_frame_end future is null. Did you remember to finish the previous frame?")
            .cleanup_finished();

        if self.should_recreate_swapchain { 
            self.recreate_swapchain(); 
            self.update_aspect_ratio(camera);
        }

        // Get an image from the swapchain, recreating the swapchain if its settings are suboptimal
        let (image_idx, suboptimal, acquire_future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                self.render_stage = RenderStage::Error;
                self.should_recreate_swapchain = true;
                return;
            },
            Err(e) => panic!("Failed to acquire next image: {:?}", e)
        };

        if suboptimal {
            // self.should_recreate_swapchain = true;
            // TODO: for some reason, swapchain is permanently suboptimal after moving to a retina display and then scaling
            println!("Swapchain is suboptimal");
        }

        // Set the clear values for each of the buffers
        let clear_values: Vec<Option<ClearValue>> = vec![
            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
            Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
            Some(ClearValue::Depth(1f32)),
        ];

        // Create a command buffer, which holds a list of commands that rell the graphics hardware what to do
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        command_buffer_builder
            .begin_render_pass(
                RenderPassBeginInfo { 
                    clear_values,
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[image_idx as usize].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap();
        
        self.commands = Some(command_buffer_builder);
        self.image_idx = image_idx;
        self.acquire_future = Some(acquire_future);
        self.render_stage = RenderStage::Albedo;
    }

    /// Finishes the rendering process and draws to the screen
    /// # Panics
    /// Panics if not called after a `draw_object_unlit()` call or a `draw_point()` call
    pub fn finish(&mut self) {
        match self.render_stage {
            RenderStage::Point => {},
            RenderStage::Unlit => {},
            RenderStage::Error => {
                self.commands = None;
                return;
            }
            _ => panic!("finish() not called in order, rendering stopped")
        }

        // End and build the render pass
        let mut command_buffer_builder = self.commands.take().unwrap();
        command_buffer_builder.end_render_pass().unwrap();
        let command_buffer = command_buffer_builder.build().unwrap();

        let af = self.acquire_future.take().unwrap();
        let fe = self.previous_frame_end.take().unwrap();

        let future = fe.join(af)
            .then_execute(self.queue.clone(), command_buffer).unwrap()
            .then_swapchain_present(self.queue.clone(), SwapchainPresentInfo::swapchain_image_index(
                self.swapchain.clone(), 
                self.image_idx
            ))
            .then_signal_fence_and_flush();
        
        match future {
            Ok(future) => {
                self.previous_frame_end = Some(Box::new(future))
            }
            Err(FlushError::OutOfDate) => {
                self.render_stage = RenderStage::Error;
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                return;
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.render_stage = RenderStage::Error;
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                return;
            }
        }

        self.commands = None;
        self.render_stage = RenderStage::Stopped;

        // TODO: In complicated programs it’s likely that one or more of the operations we’ve just scheduled 
        // will block. This happens when the graphics hardware can not accept further commands and the program 
        // has to wait until it can. Vulkan provides no easy way to check for this. Because of this, any serious 
        // application will probably want to have command submissions done on a dedicated thread so the rest of 
        // the application can keep running in the background. We will be completely ignoring this for the sake 
        // of these tutorials but just keep this in mind for your own future work.
    }
}

impl Renderer for MeshRenderer {
    type Object = MeshObject;
    
    /// Recreates the swapchain. Should be called if the swapchain is invalidated, such as by a window resize
    fn recreate_swapchain(&mut self) {
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: self.window.inner_size().into(),
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = new_swapchain;
        // TODO: use a different allocator?
        (self.framebuffers, self.attachment_buffers) = super::window_size_dependent_setup(&self.buffer_allocator, &new_images, self.render_pass.clone(), &mut self.viewport);
    }

    /// Draws an object that will later be lit
    /// # Panics
    /// Panics if not called after a `start()` call or another `draw_object()` call
    fn draw_object(&mut self, object: &mut MeshObject) -> Result<(), UnconfiguredError> {
        match self.render_stage {
            RenderStage::Albedo => {},
            RenderStage::Error => {
                self.commands = None;
                return Ok(());
            },
            _ => panic!("draw_object() not called in order, rendering stopped")
        }

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
        self.commands.as_mut().unwrap()
            .set_viewport(0, [self.viewport.clone()])
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
    fn set_ambient(&mut self, light: AmbientLight) {
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
    fn draw_ambient_light(&mut self) {
        match self.render_stage {
            RenderStage::Albedo => {
                self.render_stage = RenderStage::Ambient;
            },
            RenderStage::Error => {
                self.commands = None;
                return;
            },
            _ => panic!("draw_ambient() not called in order, rendering stopped")
        }

        if self.ambient_light_buf.is_none() { 
            self.commands.as_mut().unwrap()
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
        self.commands.as_mut().unwrap()
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
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
    fn draw_point_light(&mut self, camera: &mut Camera, point_light: &mut PointLight) {
        match self.render_stage {
            RenderStage::Ambient => {
                self.render_stage = RenderStage::Point;
            }
            RenderStage::Point => {}
            RenderStage::Error => {
                self.commands = None;
                return;
            }
            _ => panic!("draw_point() not called in order, rendering stopped")
        }

        let point_subbuffer = point_light.get_buffer(&self.point_light_buf_pool);
        let camera_pos_subbuffer = camera.get_pos_subbuffer(&self.camera_pos_buf_pool).unwrap();

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
                WriteDescriptorSet::buffer(5, camera_pos_subbuffer),
            ],
        ).unwrap();

        self.commands.as_mut().unwrap()
            .bind_pipeline_graphics(self.point_light_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.point_light_pipeline.layout().clone(),
                0,
                point_set,
            )
            .bind_vertex_buffers(0, self.dummy_vertices.clone())
            .draw(self.dummy_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }

    /// Draws an object with an unlit shader by rendering it after shadows are drawn
    /// # Panics
    /// Panics if not called after a `draw_point()` call or another `draw_object_unlit()` call
    fn draw_object_unlit(&mut self, object: &mut MeshObject) -> Result<(), UnconfiguredError> {
        match self.render_stage {
            RenderStage::Point => {
                self.render_stage = RenderStage::Unlit;
            }
            RenderStage::Unlit => {},
            RenderStage::Error => {
                self.commands = None;
                return Ok(());
            }
            _ => panic!("draw_point() not called in order, rendering stopped")
        }

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
        self.commands.as_mut().unwrap()
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
}