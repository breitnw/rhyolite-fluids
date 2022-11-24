use crate::{UnconfiguredError, vk_setup};
use crate::geometry::Object;
use crate::geometry::dummy::DummyVertex;
use crate::geometry::loader::Vertex;
use crate::shaders::{deferred_vert, directional_frag, ambient_frag, Shaders};
use crate::lighting::{AmbientLight, DirectionalLight, self};
use crate::camera::Camera;

use vulkano;
use vulkano::buffer::{CpuBufferPool, TypedBufferAccess, CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, PrimaryAutoCommandBuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo, StandardCommandBufferAlloc};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::{Device, Queue};
use vulkano::image::AttachmentImage;
use vulkano::image::view::ImageView;
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

#[derive(Debug, Clone, PartialEq)]

enum RenderStage {
    Stopped,
    Deferred,
    Ambient,
    Directional,
    RenderError,
}

pub struct Renderer {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    window: Arc<Window>,
    pub device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,

    pub buffer_allocator: Arc<GenericMemoryAllocator<Arc<FreeListAllocator>>>, // TODO: make this private
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    command_buffer_allocator: StandardCommandBufferAllocator,
    
    camera: Camera,

    ambient_buf: Option<Arc<CpuAccessibleBuffer<ambient_frag::ty::Ambient_Light_Data>>>,
    model_buf_pool: CpuBufferPool<deferred_vert::ty::Model_Data>,
    directional_buf_pool: CpuBufferPool<directional_frag::ty::Directional_Light_Data>,

    deferred_pipeline: Arc<GraphicsPipeline>,
    directional_pipeline: Arc<GraphicsPipeline>,
    ambient_pipeline: Arc<GraphicsPipeline>,

    framebuffers: Vec<Arc<Framebuffer>>,
    color_buffer: Arc<ImageView<AttachmentImage>>,
    normal_buffer: Arc<ImageView<AttachmentImage>>,

    dummy_vertices: Arc<CpuAccessibleBuffer<[DummyVertex]>>,
    viewport: Viewport,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    vp_set: Arc<PersistentDescriptorSet>,

    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>, StandardCommandBufferAllocator>>,
    image_idx: u32,
    acquire_future: Option<SwapchainAcquireFuture>,

    render_stage: RenderStage,
    should_recreate_swapchain: bool,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, mut camera: Camera) -> Self { 
        // Create the instance, the "root" object of all Vulkan operations
        let instance = crate::vk_setup::get_instance();

        let surface = WindowBuilder::new()
            .with_title("Vulkan Window")
            .with_inner_size(LogicalSize::new(300, 300))
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();
        let window = surface.object().unwrap().clone().downcast::<Window>().unwrap();

        // Get the device and physical device
        let (physical_device, device, mut queues) = crate::vk_setup::get_device(&instance, &surface);
        let queue = queues.next().unwrap();

        // Create the swapchain, an object which contains a vector of Images used for rendering and information on 
        // how to show them to the user
        let (swapchain, images) = crate::vk_setup::get_swapchain(&physical_device, &device, &surface, &window);

        let shaders = Shaders::default(&device);

        // Declare the render pass, a structure that lets us define how the rendering process should work. Tells the hardware
        // where it can expect to find input and where it can store output
        let render_pass = crate::vk_setup::get_render_pass(&device, swapchain.image_format());
        let deferred_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

        // Render pipelines
        let deferred_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(shaders.deferred_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(shaders.deferred_frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(deferred_pass)
            .build(device.clone())
            .unwrap();

        let directional_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(shaders.directional_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(shaders.directional_frag.entry_point("main").unwrap(), ())
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

        let ambient_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(shaders.ambient_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(shaders.ambient_frag.entry_point("main").unwrap(), ())
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

        // Generic allocator for framebuffer attachments, descriptor sets, vertex buffers, etc. 
        // TODO: might want to have multiple allocators separated based on function
        let buffer_allocator = Arc::from(GenericMemoryAllocator::<Arc<FreeListAllocator>>::new_default(device.clone()));
        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
        let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), StandardCommandBufferAllocatorCreateInfo::default());

        // Configure the camera
        camera.configure(&window, &buffer_allocator);
        let vp_layout = deferred_pipeline.layout().set_layouts().get(0).unwrap().clone();
        let vp_set = camera.get_vp_descriptor_set(&descriptor_set_allocator, &vp_layout).unwrap();

        // Buffers and buffer pools
        let ambient_buf = None;
        let directional_buf_pool = CpuBufferPool::<directional_frag::ty::Directional_Light_Data>::uniform_buffer(buffer_allocator.clone());
        let model_buf_pool = CpuBufferPool::<deferred_vert::ty::Model_Data>::uniform_buffer(buffer_allocator.clone());

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let (framebuffers, color_buffer, normal_buffer) = 
            crate::vk_setup::window_size_dependent_setup(&buffer_allocator, &images, render_pass.clone(), &mut viewport);

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

        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        let commands = None;
        let image_idx = 0;
        let acquire_future = None;

        let render_stage = RenderStage::Stopped;

        Renderer{ 
            instance, 
            surface, 
            window,
            device, 
            queue, 
            camera, 
            swapchain,
            render_pass,

            buffer_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,

            ambient_buf,
            directional_buf_pool,
            model_buf_pool,

            ambient_pipeline,
            directional_pipeline,
            deferred_pipeline,

            framebuffers,
            color_buffer,
            normal_buffer,

            dummy_vertices,
            vp_set,
            viewport,
            previous_frame_end,

            commands, 
            image_idx,
            acquire_future,

            render_stage,
            should_recreate_swapchain: false,
        } 
    }

    /// Recreates the swapchain. Should be called if the swapchain is invalidated, such as by a window resize
    pub fn recreate_swapchain(&mut self) {
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
        (self.framebuffers, self.color_buffer, self.normal_buffer) = 
            crate::vk_setup::window_size_dependent_setup(&self.buffer_allocator, &new_images, self.render_pass.clone(), &mut self.viewport);
    }

    /// Updates the aspect ratio of the camera. Should be called when the window is resized
    pub fn update_aspect_ratio(&mut self) {
        self.camera.configure(&self.window, &self.buffer_allocator);
        let vp_layout = self.deferred_pipeline.layout().set_layouts().get(0).unwrap().clone();
        self.vp_set = self.camera.get_vp_descriptor_set(
            &self.descriptor_set_allocator, 
            &vp_layout
        ).unwrap();
    }

    pub fn start(&mut self) {
        self.previous_frame_end.as_mut()
            .expect("previous_frame_end future is null. Did you remember to finish the previous frame?")
            .cleanup_finished();

        if self.should_recreate_swapchain { 
            self.recreate_swapchain(); 
            self.update_aspect_ratio();
        }

        // Get an image from the swapchain, recreating the swapchain if its settings are suboptimal
        let (image_idx, suboptimal, acquire_future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                self.render_stage = RenderStage::RenderError;
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

        // Set the clear color
        let clear_values: Vec<Option<ClearValue>> = vec![
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
        self.render_stage = RenderStage::Deferred;
    }


    pub fn finish(&mut self) {
        match self.render_stage {
            RenderStage::Directional => {},
            RenderStage::RenderError => {
                self.commands = None;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
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
                self.render_stage = RenderStage::RenderError;
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                return;
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.render_stage = RenderStage::RenderError;
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                return;
            }
        }

        self.commands = None;
        self.render_stage = RenderStage::Stopped;

        // std::thread::sleep(std::time::Duration::from_millis(1000 / 40))

        // TODO: In complicated programs it’s likely that one or more of the operations we’ve just scheduled 
        // will block. This happens when the graphics hardware can not accept further commands and the program 
        // has to wait until it can. Vulkan provides no easy way to check for this. Because of this, any serious 
        // application will probably want to have command submissions done on a dedicated thread so the rest of 
        // the application can keep running in the background. We will be completely ignoring this for the sake 
        // of these tutorials but just keep this in mind for your own future work.
    }


    pub fn draw_object(&mut self, object: &mut Object) -> Result<(), UnconfiguredError> {
        match self.render_stage {
            RenderStage::Deferred => {},
            RenderStage::RenderError => {
                self.commands = None;
                // TODO: err here instead of returning Ok
                return Ok(());
            },
            _ => {
                println!("calls out of order, rendering stopped");
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return Ok(());
            }
        }

        let model_subbuffer = {
            let (model_mat, normal_mat) = object.transform.get_matrices();
            let uniform_data = deferred_vert::ty::Model_Data {
                model: model_mat.into(),
                normals: normal_mat.into(),
            };
            self.model_buf_pool.from_data(uniform_data).unwrap()
        };
        let model_layout = self.deferred_pipeline.layout().set_layouts().get(1).unwrap().clone();
        let model_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            model_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, model_subbuffer)
            ]
        ).unwrap();

        // Add albedo-related commands to the command buffer
        self.commands.as_mut().unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.deferred_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.deferred_pipeline.layout().clone(), 
                0,
                (self.vp_set.clone(), model_set.clone())
            )
            // TODO: possible to bind multiple vertex buffers at once?
            .bind_vertex_buffers(0, object.vertex_buffer()?.clone())
            .draw(object.vertex_buffer()?.len() as u32, 1, 0, 0)
            .unwrap();
        
        Ok(())
    }
    
    pub fn set_ambient(&mut self, light: AmbientLight) {
        self.ambient_buf = Some(CpuAccessibleBuffer::from_data(
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

    pub fn draw_ambient(&mut self) {
        match self.render_stage {
            RenderStage::Deferred => {
                self.render_stage = RenderStage::Ambient;
            },
            RenderStage::RenderError => {
                self.commands = None;
                return;
            },
            _ => {
                println!("calls out of order, rendering stopped");
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return;
            }
        }

        if self.ambient_buf.is_none() { 
            self.commands.as_mut().unwrap()
                .next_subpass(SubpassContents::Inline)
                .unwrap();
            return; 
        }

        let ambient_layout = self.ambient_pipeline.layout().set_layouts().get(0).unwrap();
        let ambient_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            ambient_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                WriteDescriptorSet::buffer(1, self.ambient_buf.as_mut().unwrap().clone()),
            ],
        ).unwrap();

        // Add ambient light commands to the command buffer
        self.commands.as_mut().unwrap()
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.ambient_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.ambient_pipeline.layout().clone(), 
                0,
                ambient_set.clone(),
            )
            .bind_vertex_buffers(0, self.dummy_vertices.clone())
            .draw(self.dummy_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }

    pub fn draw_directional(&mut self, directional_light: &DirectionalLight) {
        match self.render_stage {
            RenderStage::Ambient => {
                self.render_stage = RenderStage::Directional;
            }
            RenderStage::Directional => { }
            RenderStage::RenderError => {
                self.commands = None;
                return;
            }
            _ => {
                println!("calls out of order, rendering stopped");
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let directional_subbuffer = lighting::generate_directional_buffer(&self.directional_buf_pool, &directional_light);
        let directional_layout = self.directional_pipeline.layout().set_layouts().get(0).unwrap().clone();
        let directional_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            directional_layout.clone(),
            [ 
                WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                WriteDescriptorSet::image_view(1, self.normal_buffer.clone()),
                WriteDescriptorSet::buffer(2, directional_subbuffer)
            ],
        ).unwrap();
        self.commands.as_mut().unwrap()
            .bind_pipeline_graphics(self.directional_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.directional_pipeline.layout().clone(),
                0,
                directional_set.clone(),
            )
            .bind_vertex_buffers(0, self.dummy_vertices.clone())
            .draw(self.dummy_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }
}