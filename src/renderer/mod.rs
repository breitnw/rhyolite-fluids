use vulkano::{self, sync, swapchain};

use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, CommandBufferUsage, RenderPassBeginInfo, SubpassContents};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAlloc, StandardCommandBufferAllocatorCreateInfo};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::Framebuffer;
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainAcquireFuture, SwapchainCreationError, AcquireError, SwapchainPresentInfo};
use vulkano::Version;
use vulkano::format::ClearValue;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;

pub mod mesh;
pub mod marched;

pub trait Renderer {
    type Object;
    fn recreate_swapchain_and_buffers(&mut self);
}

pub struct RenderBase {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    window: Arc<Window>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<SwapchainImage>>,

    command_buffer_allocator: StandardCommandBufferAllocator,

    viewport: Viewport,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>, StandardCommandBufferAllocator>>,
    image_idx: u32,
    acquire_future: Option<SwapchainAcquireFuture>,

    should_recreate_swapchain: bool,
    render_error: bool,
}

impl RenderBase {
    pub fn new(event_loop: &EventLoop<()>) -> Self { 
        // Create the instance, the "root" object of all Vulkan operations
        let instance = get_instance();

        let surface = WindowBuilder::new()
            .with_title("Vulkan Window")
            .with_inner_size(LogicalSize::new(300, 300))
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();
        let window = surface.object().unwrap().clone().downcast::<Window>().unwrap();

        // Get the device and physical device
        let (physical_device, device, mut queues) = get_device(&instance, &surface);
        let queue = queues.next().unwrap();

        // Create the swapchain, an object which contains a vector of Images used for rendering and information on 
        // how to show them to the user
        let (swapchain, images) = get_swapchain(&physical_device, &device, &surface, &window);

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), StandardCommandBufferAllocatorCreateInfo::default());

        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        let commands = None;
        let image_idx = 0;
        let acquire_future = None;

        Self{ 
            instance, 
            surface, 
            window,
            device, 
            queue,  
            swapchain,
            images,

            viewport,
            previous_frame_end,

            commands, 
            image_idx,
            acquire_future,

            command_buffer_allocator,

            should_recreate_swapchain: false,
            render_error: false,
        } 
    }
    

    /// Starts the rendering process for the current frame
    fn start(&mut self, framebuffers: &Vec<Arc<Framebuffer>>) {

        self.previous_frame_end.as_mut()
            .expect("previous_frame_end future is null. Did you remember to finish the previous frame?")
            .cleanup_finished();

        // Get an image from the swapchain, recreating the swapchain if its settings are suboptimal
        let (image_idx, suboptimal, acquire_future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                self.should_recreate_swapchain = true;
                self.render_error = true;
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
                    ..RenderPassBeginInfo::framebuffer(framebuffers[image_idx as usize].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap();
        
        self.commands = Some(command_buffer_builder);
        self.image_idx = image_idx;
        self.acquire_future = Some(acquire_future);

        let viewport = self.viewport.clone();
        self.commands_mut().set_viewport(0, [viewport]);
    }
    
    /// Finishes the rendering process and draws to the screen
    /// # Panics
    /// Panics if not called after a `draw_object_unlit()` call or a `draw_point()` call
    fn finish(&mut self) {

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
                self.render_error = true;
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                return;
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.render_error = true;
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())));
                return;
            }
        }

        self.commands = None;

        // TODO: In complicated programs it’s likely that one or more of the operations we’ve just scheduled 
        // will block. This happens when the graphics hardware can not accept further commands and the program 
        // has to wait until it can. Vulkan provides no easy way to check for this. Because of this, any serious 
        // application will probably want to have command submissions done on a dedicated thread so the rest of 
        // the application can keep running in the background. We will be completely ignoring this for the sake 
        // of these tutorials but just keep this in mind for your own future work.
    }

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
        self.images = new_images;
    }

    fn commands_mut(&mut self) -> &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>, StandardCommandBufferAllocator> {
        // Should never panic because commands is initialized in start()
        self.commands.as_mut().unwrap()
    }
}

// HELPER FUNCTIONS FOR RenderBase CREATION

/// Selects the best physical device based on the available hardware.
pub(crate) fn select_physical_device(
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

pub(crate) fn get_instance() -> Arc<Instance> {
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
}

pub(crate) fn get_device(instance: &Arc<Instance>, surface: &Arc<Surface>) -> (Arc<PhysicalDevice>, Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>) {
    // Specify features for the physical device with the relevant extensions
    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    let (physical_device, queue_family_index) =
        select_physical_device(instance, surface, &device_ext);

    // Create a device, which is the software representation of the hardware stored in the physical device
    let (device, queues) = Device::new(
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

    (physical_device, device, queues)
}

pub(crate) fn get_swapchain(
    physical_device: &Arc<PhysicalDevice>, 
    device: &Arc<Device>, 
    surface: &Arc<Surface>, 
    window: &Arc<Window>
) -> (Arc<Swapchain>, Vec<Arc<SwapchainImage>>) {
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
}