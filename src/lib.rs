#![allow(dead_code)]

use vulkano;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage, TypedBufferAccess, CpuBufferPool};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents, RenderPassBeginInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::AttachmentImage;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::memory::allocator::{GenericMemoryAllocator, FreeListAllocator};
use vulkano::pipeline::graphics::rasterization::{RasterizationState, CullMode};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::render_pass::{Framebuffer,  RenderPass, Subpass};
use vulkano::swapchain::{
    self, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::Version;
use vulkano::format::{ClearValue, Format};

use vulkano_win::VkSurfaceBuild;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;
use std::time::Instant;

mod geometry;
use geometry::{Vertex, MVP};

mod lighting;
use lighting::{AmbientLight, DirectionalLight};

mod shaders;
mod vk_setup;

// TODO: implement frames in flight if not implemented in the tutorial

pub struct Rhyolite {
    renderer: Renderer,
}
impl Rhyolite {
    pub fn new() -> Self {
        Rhyolite {
            renderer: Renderer::new()
        }
    }
    pub fn run(self) {
        self.renderer.run();
    }
}

struct Renderer {
    device: Arc<Device>,
    event_loop: EventLoop<()>,
    swapchain: Arc<Swapchain>,
    window: Arc<Window>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,

    framebuffers: Vec<Arc<Framebuffer>>,
    color_buffer: Arc<ImageView<AttachmentImage>>,
    normal_buffer: Arc<ImageView<AttachmentImage>>,

    command_buffer_allocator: StandardCommandBufferAllocator,
    generic_allocator: Arc<GenericMemoryAllocator<Arc<FreeListAllocator>>>,
    queue: Arc<Queue>,

    deferred_pipeline: Arc<GraphicsPipeline>,
    lighting_pipeline: Arc<GraphicsPipeline>,
}

impl Renderer {
    // TODO: seperate new and run functions
    fn new() -> Self {
        // Create the instance, the "root" object of all Vulkan operations
        let instance = {
            let library = VulkanLibrary::new().unwrap();
            let required_extensions = vulkano_win::required_extensions(&*library);
            Instance::new(
                library,
                InstanceCreateInfo {
                    enabled_extensions: required_extensions,
                    enumerate_portability: true,
                    max_api_version: Some(Version::V1_1),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        // Create the basic window and event loop
        let event_loop = EventLoop::new();
        let surface = WindowBuilder::new()
            .with_title("Vulkan Window")
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();

        let window = surface.object().unwrap().clone().downcast::<Window>().unwrap();

        // Specify features for the physical device with the relevant extensions
        let device_ext = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        // Get the physical device using these features
        let (physical_device, queue_family_index) =
            vk_setup::select_physical_device(&instance, &surface, &device_ext);

        // Create a device, which is the software representation of the hardware stored in the physical device
        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_ext,
                ..Default::default()
            },
        )
        .unwrap();

        // For now, we'll just use a singular queue
        let queue = queues.next().unwrap();

        // Create the swapchain, an object which contains a vector of Images used for rendering and information on 
        // how to show them to the user
        let (swapchain, images) = {
            let caps = physical_device
                .surface_capabilities(&surface, Default::default())
                .unwrap();
            let usage = caps.supported_usage_flags;
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let image_format = Some(
                physical_device
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0
            );
            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count, // TODO: +1?
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: usage,
                    composite_alpha: alpha,
                    ..Default::default()
                }
            )
            .unwrap()
        };

        // Declare the render pass, a structure that lets us define how the rendering process should work. Tells the hardware
        // where it can expect to find input and where it can store output
        let render_pass = vulkano::ordered_passes_renderpass!(
            device.clone(),
            attachments: {
                final_color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                color: {
                    load: Clear,
                    store: Store,
                    format: Format::A2B10G10R10_UNORM_PACK32,
                    samples: 1,
                },
                normals: {
                    load: Clear,
                    store: DontCare,
                    format: Format::R16G16B16A16_SFLOAT,
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
                    color: [color, normals],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [final_color],
                    depth_stencil: {},
                    input: [color, normals]
                }
            ]
        )
        .unwrap();

        let deferred_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

        let deferred_vert = shaders::deferred_vert::load(device.clone()).unwrap();
        let deferred_frag = shaders::deferred_frag::load(device.clone()).unwrap();
        let lighting_vert = shaders::lighting_vert::load(device.clone()).unwrap();
        let lighting_frag = shaders::lighting_frag::load(device.clone()).unwrap();

        let deferred_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(deferred_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(deferred_frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(deferred_pass)
            .build(device.clone())
            .unwrap();

        let lighting_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(lighting_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(lighting_frag.entry_point("main").unwrap(), ())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass)
            .build(device.clone())
            .unwrap();
        
        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        // A generic FreeListAllocator used to allocate the vertex buffer and other data
        // TODO: might want to have multiple allocators separated based on function
        let generic_allocator = Arc::from(
            GenericMemoryAllocator::<Arc<FreeListAllocator>>::new_default(device.clone())
        );

        let (framebuffers, color_buffer, normal_buffer) = 
            vk_setup::window_size_dependent_setup(&generic_allocator, &images, render_pass.clone(), &mut viewport);

        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(), 
            StandardCommandBufferAllocatorCreateInfo::default()
        );

        Self {
            device,
            event_loop,
            swapchain,
            window,
            render_pass,
            viewport,

            framebuffers,
            color_buffer,
            normal_buffer,

            command_buffer_allocator,
            generic_allocator,
            queue,

            deferred_pipeline,
            lighting_pipeline,

        }
    }

    pub fn run(mut self) {
        let vertex_buf = CpuAccessibleBuffer::from_iter(
            &self.generic_allocator,
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
        ).unwrap();

        let uniform_buf = CpuBufferPool::<shaders::deferred_vert::ty::MVP_Data>::uniform_buffer(self.generic_allocator.clone());
        let ambient_light_buf = CpuBufferPool::<shaders::lighting_frag::ty::Ambient_Light_Data>::uniform_buffer(self.generic_allocator.clone());
        let directional_light_buf = CpuBufferPool::<shaders::lighting_frag::ty::Directional_Light_Data>::uniform_buffer(self.generic_allocator.clone());

        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(self.device.clone());

        // Lighting
        let ambient_light = AmbientLight {
            color: [1.0, 1.0, 1.0],
            intensity: 0.2
        };
        let directional_light = DirectionalLight {
            position: [-4.0, -4.0, 0.0, 1.0], 
            color: [1.0, 1.0, 1.0]
        };
        
        // Running the code
        let mut recreate_swapchain = false;
        let mut previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<dyn GpuFuture>);

        let rotation_start = Instant::now();

        // TODO: "For more fully-featured applications you’ll want to decouple program logic (for instance, simulating 
        // a game’s economy) from rendering operations."
        self.event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                },
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    recreate_swapchain = true;
                },
                Event::RedrawEventsCleared => {
                    previous_frame_end.as_mut().take().unwrap().cleanup_finished();

                    let uniform_subbuffer = {
                        let elapsed = rotation_start.elapsed().as_secs_f32();
                        let dimensions: [u32; 2] = self.window.inner_size().into();
                        let mvp = MVP::perspective(dimensions[0] as f32 / dimensions[1] as f32, elapsed);
                        let uniform_data = shaders::deferred_vert::ty::MVP_Data {
                            model: mvp.model.into(),
                            view: mvp.view.into(),
                            projection: mvp.projection.into(),
                        };
                        uniform_buf.from_data(uniform_data).unwrap()
                    };

                    let ambient_subbuffer = {
                        let uniform_data = shaders::lighting_frag::ty::Ambient_Light_Data {
                            color: ambient_light.color.into(),
                            intensity: ambient_light.intensity.into()
                        };
                        ambient_light_buf.from_data(uniform_data).unwrap()
                    };

                    let directional_subbuffer = {
                        let uniform_data = shaders::lighting_frag::ty::Directional_Light_Data {
                            color: directional_light.color.into(),
                            position: directional_light.position.into()
                        };
                        directional_light_buf.from_data(uniform_data).unwrap()
                    };

                    let deferred_layout = self.deferred_pipeline
                        .layout()
                        .set_layouts()
                        .get(0)
                        .unwrap();
                    // TODO: use a different descriptor set because PersistentDescriptorSet is expected to be long-lived
                    let deferred_set = PersistentDescriptorSet::new(
                        &descriptor_set_allocator,
                        deferred_layout.clone(),
                        [
                            WriteDescriptorSet::buffer(0, uniform_subbuffer.clone()),
                            // WriteDescriptorSet::buffer(1, ambient_subbuffer),
                            // WriteDescriptorSet::buffer(2, directional_subbuffer),
                        ],
                    ).unwrap();

                    let lighting_layout = self.lighting_pipeline
                        .layout()
                        .set_layouts()
                        .get(0)
                        .unwrap();
                    // TODO: use a different descriptor set because PersistentDescriptorSet is expected to be long-lived
                    let lighting_set = PersistentDescriptorSet::new(
                        &descriptor_set_allocator,
                        lighting_layout.clone(),
                        [
                            WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                            WriteDescriptorSet::image_view(1, self.normal_buffer.clone()),
                            WriteDescriptorSet::buffer(2, uniform_subbuffer.clone()),
                            WriteDescriptorSet::buffer(3, ambient_subbuffer),
                            WriteDescriptorSet::buffer(4, directional_subbuffer)
                        ],
                    ).unwrap();


                    // Recreate the swapchain if it was invalidated, such as by a window resize
                    if recreate_swapchain {
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
                            vk_setup::window_size_dependent_setup(&self.generic_allocator, &new_images, self.render_pass.clone(), &mut self.viewport);
                        recreate_swapchain = false;
                    }

                    // Get an image from the swapchain, recreating the swapchain if its settings are suboptimal
                    let (image_num, suboptimal, acquire_future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        },
                        Err(e) => panic!("Failed to acquire next image: {:?}", e)
                    };

                    if suboptimal {
                        recreate_swapchain = true;
                    }

                    // Set the clear color
                    let clear_values: Vec<Option<ClearValue>> = vec![
                        Some(ClearValue::Float([0.5, 0.0, 1.0, 1.0])),
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::Depth(1f32)),
                    ];

                    // Create a command buffer, which holds a list of commands that rell the graphics hardware what to do
                    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
                        &self.command_buffer_allocator,
                        self.queue.queue_family_index(),
                        CommandBufferUsage::OneTimeSubmit,
                    )
                    .unwrap();

                    command_buffer_builder
                        .begin_render_pass(
                            RenderPassBeginInfo { 
                                clear_values,
                                ..RenderPassBeginInfo::framebuffer(self.framebuffers[image_num as usize].clone())
                            },
                            SubpassContents::Inline,
                        )
                        .unwrap()
                        .set_viewport(0, [self.viewport.clone()])
                        .bind_pipeline_graphics(self.deferred_pipeline.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics, 
                            self.deferred_pipeline.layout().clone(), 
                            0,
                            deferred_set.clone()
                        )
                        .bind_vertex_buffers(0, vertex_buf.clone())
                        .draw(vertex_buf.len() as u32, 1, 0, 0)
                        .unwrap()
                        .next_subpass(SubpassContents::Inline)
                        .unwrap()
                        .bind_pipeline_graphics(self.lighting_pipeline.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics, 
                            self.lighting_pipeline.layout().clone(), 
                            0, 
                            lighting_set.clone()
                        )
                        .draw(vertex_buf.len() as u32, 1, 0, 0)
                        .unwrap()
                        .end_render_pass()
                        .unwrap();
                    
                    let command_buffer = command_buffer_builder.build().unwrap();

                    let future = previous_frame_end
                        .take()
                        .unwrap()
                        .join(acquire_future)
                        .then_execute(self.queue.clone(), command_buffer)
                        .unwrap()
                        .then_swapchain_present(self.queue.clone(), SwapchainPresentInfo::swapchain_image_index(
                            self.swapchain.clone(), 
                            image_num
                        ))
                        .then_signal_fence_and_flush();

                    match future {
                        Ok(future) => {
                            previous_frame_end = Some(Box::new(future));
                        }
                        Err(FlushError::OutOfDate) => {
                            recreate_swapchain = true;
                            previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                        }
                        Err(e) => {
                            println!("Failed to flush future: {:?}", e);
                            previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                        }
                    }

                    // TODO: In complicated programs it’s likely that one or more of the operations we’ve just scheduled 
                    // will block. This happens when the graphics hardware can not accept further commands and the program 
                    // has to wait until it can. Vulkan provides no easy way to check for this. Because of this, any serious 
                    // application will probably want to have command submissions done on a dedicated thread so the rest of 
                    // the application can keep running in the background. We will be completely ignoring this for the sake 
                    // of these tutorials but just keep this in mind for your own future work.
                },
                _ => {}
            }
        });
    }
}
