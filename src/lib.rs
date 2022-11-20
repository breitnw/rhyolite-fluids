#![allow(dead_code)]

use geometry::Object;
use geometry::dummy::DummyVertex;
use nalgebra_glm::{translate, identity, rotate_z, rotate_x, rotate_y, vec3};
use shaders::{deferred_vert, deferred_frag, directional_vert, directional_frag, ambient_frag, ambient_vert};
use transform::Transform;
use vulkano;

use vulkano::buffer::{CpuBufferPool, TypedBufferAccess, CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::memory::allocator::{GenericMemoryAllocator, FreeListAllocator};
use vulkano::pipeline::graphics::color_blend::{ColorBlendState, BlendFactor, AttachmentBlend, BlendOp};
use vulkano::pipeline::graphics::rasterization::{RasterizationState, CullMode};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::render_pass::Subpass;
use vulkano::swapchain::{
    self, AcquireError, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::format::ClearValue;

use vulkano_win::VkSurfaceBuild;

use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;
use std::time::Instant;

mod geometry;
use geometry::loader::{Vertex, ModelBuilder};

mod lighting;
use lighting::{AmbientLight, DirectionalLight};

mod camera;
use camera::Camera;

mod shaders;
mod vk_setup;
mod transform;


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

struct Renderer {}

impl Renderer {
    fn new() -> Self { Renderer{} }

    // TODO: seperate new and run functions
    fn run(&self) {
        // Create the instance, the "root" object of all Vulkan operations
        let instance = vk_setup::get_instance();

        // Create the basic window and event loop
        let event_loop = EventLoop::new();
        let surface = WindowBuilder::new()
            .with_title("Vulkan Window")
            .with_inner_size(LogicalSize::new(300, 300))
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();

        let window = surface.object().unwrap().clone().downcast::<Window>().unwrap();

        // Get the device and physical device
        let (physical_device, device, mut queues) = vk_setup::get_device(&instance, &surface);

        // For now, we'll just use a singular queue
        let queue = queues.next().unwrap();

        // Create the swapchain, an object which contains a vector of Images used for rendering and information on 
        // how to show them to the user
        let (mut swapchain, images) = vk_setup::get_swapchain(&physical_device, &device, &surface, &window);

        // Declare the render pass, a structure that lets us define how the rendering process should work. Tells the hardware
        // where it can expect to find input and where it can store output
        let render_pass = vk_setup::get_render_pass(&device, swapchain.image_format());

        let deferred_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

        let deferred_vert = deferred_vert::load(device.clone()).unwrap();
        let deferred_frag = deferred_frag::load(device.clone()).unwrap();
        let directional_vert = directional_vert::load(device.clone()).unwrap();
        let directional_frag = directional_frag::load(device.clone()).unwrap();
        let ambient_vert = ambient_vert::load(device.clone()).unwrap();
        let ambient_frag = ambient_frag::load(device.clone()).unwrap();


        // RENDER PIPELINES
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

        let directional_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(directional_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(directional_frag.entry_point("main").unwrap(), ())
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
            .vertex_shader(ambient_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(ambient_frag.entry_point("main").unwrap(), ())
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
        
        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };


        // A generic FreeListAllocator used to allocate the vertex buffer and other data
        // TODO: might want to have multiple allocators separated based on function
        let generic_allocator = Arc::from(GenericMemoryAllocator::<Arc<FreeListAllocator>>::new_default(device.clone()));

        let (mut framebuffers, mut color_buffer, mut normal_buffer) = 
            vk_setup::window_size_dependent_setup(&generic_allocator, &images, render_pass.clone(), &mut viewport);

        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(), 
            StandardCommandBufferAllocatorCreateInfo::default()
        );


        // Create the camera
        let camera_transform = Transform::new();
        let mut camera = Camera::new(camera_transform, window.inner_size().into(), 1.2, 0.02, 100.0);


        // Create a dummy vertex buffer used for full-screen shaders
        let dummy_vertices = CpuAccessibleBuffer::from_iter(
            &generic_allocator, 
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            }, 
            false,
            DummyVertex::list().into_iter(),
        ).unwrap();


        // Build the model
        let vertices = ModelBuilder::from_file("data/models/monkey_smooth.obj", true).build_with_color([1.0, 1.0, 1.0]);
        let mut object_transform = Transform::new();
        object_transform.set_translation_mat(translate(&identity(), &vec3(0.0, 0.0, -5.0)));
        let mut object = Object::new(object_transform, vertices, &generic_allocator);


        // Lighting
        let ambient_light = AmbientLight {
            color: [1.0, 1.0, 1.0],
            intensity: 0.2
        };
        let directional_lights = vec![
            DirectionalLight {position: [-4.0, 0.0, -2.0, 1.0], color: [1.0, 0.0, 0.0]},
            DirectionalLight {position: [0.0, -4.0, 1.0, 1.0], color: [0.0, 1.0, 0.0]},
            DirectionalLight {position: [4.0, -2.0, -1.0, 1.0], color: [0.0, 0.0, 1.0]},
        ];


        // BUFFERS AND DESCRIPTORS
        // Buffers
        let ambient_buf = CpuBufferPool::<ambient_frag::ty::Ambient_Light_Data>::uniform_buffer(generic_allocator.clone());
        let directional_buf = CpuBufferPool::<directional_frag::ty::Directional_Light_Data>::uniform_buffer(generic_allocator.clone());
        let model_buf = CpuBufferPool::<deferred_vert::ty::Model_Data>::uniform_buffer(generic_allocator.clone());

        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());

        // Layouts
        let vp_layout = deferred_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone();

        let model_layout = deferred_pipeline
            .layout()
            .set_layouts()
            .get(1)
            .unwrap()
            .clone();

        let ambient_layout = ambient_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone();

        let directional_layout = directional_pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone();

        // Persistent descriptor sets
        let mut vp_set = camera.get_vp_descriptor_set(
            &generic_allocator, 
            &descriptor_set_allocator, 
            &vp_layout
        );

        
        // Time
        let mut t: f32 = 0.0;
        let time_start = Instant::now();
        

        // Running the code
        let mut recreate_swapchain = false;
        let mut aspect_ratio_changed = false;
        let mut previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        // TODO: "For more fully-featured applications you’ll want to decouple program logic (for instance, simulating 
        // a game’s economy) from rendering operations."
        event_loop.run(move |event, _, control_flow| {
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
                    aspect_ratio_changed = true;

                },
                Event::RedrawEventsCleared => {
                    previous_frame_end.as_mut().take().unwrap().cleanup_finished();

                    // Update time-related variables
                    let prev_t = t;
                    t = time_start.elapsed().as_secs_f32();
                    let delta = t - prev_t;

                    // Update the object's transform
                    object.transform.set_translation_mat(translate(&identity(), &vec3(0.0, t.sin(), -5.0)));
                    object.transform.set_rotation_mat({
                        let mut rotation = identity();
                        rotation = rotate_y(&rotation, t);
                        rotation = rotate_x(&rotation, t / 2.);
                        rotation = rotate_z(&rotation, t / 3.);
                        rotation
                    });

                    let model_subbuffer = {
                        let (model_mat, normal_mat) = object.transform.get_matrices();
                        let uniform_data = deferred_vert::ty::Model_Data {
                            model: model_mat.into(),
                            normals: normal_mat.into(),
                        };
                        model_buf.from_data(uniform_data).unwrap()
                    };
                    let model_set = PersistentDescriptorSet::new(
                        &descriptor_set_allocator,
                        model_layout.clone(),
                        [
                            WriteDescriptorSet::buffer(0, model_subbuffer)
                        ]
                    ).unwrap();

                    let ambient_subbuffer = {
                        let uniform_data = ambient_frag::ty::Ambient_Light_Data {
                            color: ambient_light.color.into(),
                            intensity: ambient_light.intensity.into()
                        };
                        ambient_buf.from_data(uniform_data).unwrap()
                    };
                    // TODO: use a different descriptor set because PersistentDescriptorSet is expected to be long-lived
                    let ambient_set = PersistentDescriptorSet::new(
                        &descriptor_set_allocator,
                        ambient_layout.clone(),
                        [
                            WriteDescriptorSet::image_view(0, color_buffer.clone()),
                            WriteDescriptorSet::buffer(1, ambient_subbuffer),
                        ],
                    ).unwrap();                    

                    if aspect_ratio_changed {
                        camera.update_aspect_ratio(window.inner_size().into());
                        vp_set = camera.get_vp_descriptor_set(
                            &generic_allocator, 
                            &descriptor_set_allocator, 
                            &vp_layout
                        );
                    }

                    // Recreate the swapchain if it was invalidated, such as by a window resize
                    if recreate_swapchain {
                        let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                            image_extent: window.inner_size().into(),
                            ..swapchain.create_info()
                        }) {
                            Ok(r) => r,
                            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };

                        swapchain = new_swapchain;
                        // TODO: use a different allocator?
                        (framebuffers, color_buffer, normal_buffer) = 
                            vk_setup::window_size_dependent_setup(&generic_allocator, &new_images, render_pass.clone(), &mut viewport);
                        recreate_swapchain = false;
                    }

                    // Get an image from the swapchain, recreating the swapchain if its settings are suboptimal
                    let (image_num, suboptimal, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
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
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                        Some(ClearValue::Depth(1f32)),
                    ];

                    // Create a command buffer, which holds a list of commands that rell the graphics hardware what to do
                    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
                        &command_buffer_allocator,
                        queue.queue_family_index(),
                        CommandBufferUsage::OneTimeSubmit,
                    ).unwrap();

                    // Add albedo-related commands to the command buffer
                    command_buffer_builder
                        .begin_render_pass(
                            RenderPassBeginInfo { 
                                clear_values,
                                ..RenderPassBeginInfo::framebuffer(framebuffers[image_num as usize].clone())
                            },
                            SubpassContents::Inline,
                        )
                        .unwrap()
                        .set_viewport(0, [viewport.clone()])
                        .bind_pipeline_graphics(deferred_pipeline.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics, 
                            deferred_pipeline.layout().clone(), 
                            0,
                            (vp_set.clone(), model_set.clone())
                        )
                        .bind_vertex_buffers(0, object.vertex_buffer().clone())
                        .draw(object.vertex_buffer().len() as u32, 1, 0, 0)
                        .unwrap()
                        .next_subpass(SubpassContents::Inline)
                        .unwrap();

                    // Record directional lights into the command buffer
                    for d_light in directional_lights.iter() {
                        let directional_subbuffer = crate::lighting::generate_directional_buffer(&directional_buf, &d_light);
                        let directional_set = PersistentDescriptorSet::new(
                            &descriptor_set_allocator,
                            directional_layout.clone(),
                            [ 
                                WriteDescriptorSet::image_view(0, color_buffer.clone()),
                                WriteDescriptorSet::image_view(1, normal_buffer.clone()),
                                WriteDescriptorSet::buffer(2, directional_subbuffer)
                            ],
                        ).unwrap();
                        command_buffer_builder
                            .bind_pipeline_graphics(directional_pipeline.clone())
                            .bind_descriptor_sets(
                                PipelineBindPoint::Graphics,
                                directional_pipeline.layout().clone(),
                                0,
                                directional_set.clone(),
                            )
                            .bind_vertex_buffers(0, dummy_vertices.clone())
                            .draw(dummy_vertices.len() as u32, 1, 0, 0)
                            .unwrap();
                    }

                    // Record ambient lights into the command buffer
                    command_buffer_builder
                        .bind_pipeline_graphics(ambient_pipeline.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            ambient_pipeline.layout().clone(),
                            0,
                            ambient_set.clone(),
                        )
                        .bind_vertex_buffers(0, dummy_vertices.clone())
                        .draw(dummy_vertices.len() as u32, 1, 0, 0)
                        .unwrap();

                    // End the render pass
                    command_buffer_builder
                        .end_render_pass()
                        .unwrap();

                    // Build the command buffer
                    let command_buffer = command_buffer_builder.build().unwrap();

                    let future = previous_frame_end
                        .take()
                        .unwrap()
                        .join(acquire_future)
                        .then_execute(queue.clone(), command_buffer)
                        .unwrap()
                        .then_swapchain_present(queue.clone(), SwapchainPresentInfo::swapchain_image_index(
                            swapchain.clone(), 
                            image_num
                        ))
                        .then_signal_fence_and_flush();

                    match future {
                        Ok(future) => {
                            previous_frame_end = Some(Box::new(future));
                        }
                        Err(FlushError::OutOfDate) => {
                            recreate_swapchain = true;
                            previous_frame_end = Some(Box::new(sync::now(device.clone())));
                        }
                        Err(e) => {
                            println!("Failed to flush future: {:?}", e);
                            previous_frame_end = Some(Box::new(sync::now(device.clone())));
                        }
                    }

                    // std::thread::sleep(std::time::Duration::from_millis(1000 / 40))

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
