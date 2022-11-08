#![allow(dead_code)]

use vulkano;

use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents, RenderPassBeginInfo};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{
    self, AcquireError, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano::Version;
use vulkano::format::ClearValue;

use vulkano_win::VkSurfaceBuild;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;

mod geometry;

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
    command_buffer_allocator: StandardCommandBufferAllocator,
    queue: Arc<Queue>,

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
            Renderer::select_physical_device(&instance, &surface, &device_ext);

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
        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap();

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let framebuffers = Renderer::window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

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
            command_buffer_allocator,
            queue,
        }
    }

    pub fn run(mut self) {
        // Running the code
        let mut recreate_swapchain = false;
        let mut previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<dyn GpuFuture>);

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
                        self.framebuffers = Renderer::window_size_dependent_setup(&new_images, self.render_pass.clone(), &mut self.viewport);
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

    /// Selects the best physical device based on the available hardware
    fn select_physical_device(
        instance: &Arc<Instance>,
        surface: &Arc<Surface>,
        device_extensions: &DeviceExtensions,
    ) -> (Arc<PhysicalDevice>, u32) {
        let (physical_device, queue_family) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.supported_extensions().contains(device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .find(|&q| {
                        q.1.queue_flags.graphics
                            && p.surface_support(q.0 as u32, surface).unwrap_or(false)
                    })
                    .map(|q| (p.clone(), q.0))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
            .unwrap();
        (physical_device, queue_family as u32)
    }

    /// Sets up the framebuffers based on the size of the viewport
    fn window_size_dependent_setup(
        images: &[Arc<SwapchainImage>],
        render_pass: Arc<RenderPass>,
        viewport: &mut Viewport,
    ) -> Vec<Arc<Framebuffer>> {
        let dimensions = images[0].dimensions().width_height();
        viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];
        images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    }
                )
                .unwrap()
            })
            .collect()
    }
}
