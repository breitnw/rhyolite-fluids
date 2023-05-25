use vulkano::{self, swapchain, sync};

use vulkano::command_buffer::allocator::{
    StandardCommandBufferAlloc, StandardCommandBufferAllocator,
    StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
    SubpassContents,
};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::format::ClearValue;
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::Framebuffer;
use vulkano::swapchain::{
    AcquireError, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::{FlushError, GpuFuture};
use vulkano::Version;
use vulkano_win;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;
use winit::dpi::LogicalSize;

#[cfg(feature = "marched")]
pub mod marched;
#[cfg(feature = "mesh")]
pub mod mesh;
pub mod staging;

pub trait Renderer {
    fn recreate_all_size_dependent(&mut self);
    fn get_base(&self) -> &RenderBase;
    fn get_window_size(&self) -> [i32; 2] {
        self.get_base().window.inner_size().into()
    }
}

/// A struct representing the essential elements of any rendering engine created with Rhyolite.
///
/// The `RenderBase` takes care of the following responsibilities:
/// - Basic setup of Vulkan structs, including the Vulkan instance, physical and logical devices,
/// surface, queues, swapchain, viewport, and command buffers
/// - GPU synchronization
/// - Swapchain recreation (if necessary)
/// - Management and execution of command buffers
pub struct RenderBase {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    window: Arc<Window>,
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    pub(crate) images: Vec<Arc<SwapchainImage>>,

    graphics_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,

    command_buffer_allocator: StandardCommandBufferAllocator,

    viewport: Viewport,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    commands: Option<
        AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>,
            StandardCommandBufferAllocator,
        >,
    >,
    image_idx: u32,
    acquire_future: Option<SwapchainAcquireFuture>,

    should_recreate_swapchain: bool,
    render_error: bool,
}

impl RenderBase {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        // Create the instance, the "root" object of all Vulkan operations
        let instance = get_instance();

        let window = Arc::from(
            WindowBuilder::new()
                .with_title(env!("CARGO_PKG_NAME"))
                .with_inner_size(LogicalSize::new(250, 250))
                // .with_inner_size(LogicalSize::new(1280, 720))
                .build(event_loop)
                .unwrap(),
        );

        let surface =
            vulkano_win::create_surface_from_winit(window.clone(), instance.clone()).unwrap();

        // Get the device and physical device
        let (physical_device, device, queues) = get_device(&instance, &surface);

        let queues: Vec<Arc<Queue>> = queues.collect();

        let find_queue = |queue_flags: QueueFlags| -> Arc<Queue> {
            queues
                .iter()
                .find(|q| {
                    physical_device.queue_family_properties()[q.queue_family_index() as usize]
                        .queue_flags
                        .contains(queue_flags)
                })
                .unwrap()
                .clone()
        };

        let graphics_queue = find_queue(QueueFlags::GRAPHICS);
        let transfer_queue = find_queue(QueueFlags::TRANSFER);

        println!(
            "Queue families:\n\tQueueFlags::GRAPHICS: {}\n\tQueueFlags::TRANSFER: {}",
            graphics_queue.queue_family_index(),
            transfer_queue.queue_family_index()
        );

        // Create the swapchain, an object which contains a vector of Images used for rendering and information on
        // how to show them to the user
        let (swapchain, images) = get_swapchain(&physical_device, &device, &surface, &window);

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );

        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        let commands = None;
        let image_idx = 0;
        let acquire_future = None;

        Self {
            instance,
            surface,
            window,
            device,
            swapchain,
            images,

            graphics_queue,
            transfer_queue,

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
        self.previous_frame_end
            .as_mut()
            .expect(
                "previous_frame_end future is null. Did you remember to finish the previous frame?",
            )
            .cleanup_finished();

        // Get an image from the swapchain, recreating the swapchain if its settings are suboptimal
        let (image_idx, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.should_recreate_swapchain = true;
                    self.render_error = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
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

        self.image_idx = image_idx;
        self.acquire_future = Some(acquire_future);

        let viewport = self.viewport.clone();

        self.commands_mut()
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values,
                    ..RenderPassBeginInfo::framebuffer(framebuffers[image_idx as usize].clone())
                },
                SubpassContents::Inline,
            )
            .unwrap()
            .set_viewport(0, [viewport]);
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

        let future = fe
            .join(af)
            .then_execute(self.graphics_queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.graphics_queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), self.image_idx),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => self.previous_frame_end = Some(Box::new(future)),
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

    /// Gets a mutable reference to the current command buffer, which holds a list of commands that
    /// tell the graphics hardware what to do. If no such buffer yet exists, the function will
    /// create a new one.
    pub fn commands_mut(
        &mut self,
    ) -> &mut AutoCommandBufferBuilder<
        PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>,
        StandardCommandBufferAllocator,
    > {
        let cbb = match self.commands.take() {
            None => {
                 AutoCommandBufferBuilder::primary(
                    &self.command_buffer_allocator,
                    self.graphics_queue.queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit,
                ).unwrap()
            }
            Some(current_cbb) => { current_cbb }
        };
        self.commands = Some(cbb);
        self.commands.as_mut().unwrap()
    }

    pub fn get_device(&self) -> Arc<Device> { self.device.clone() }
    pub fn get_viewport(&self) -> &Viewport { &self.viewport }
}

// ========================================
// HELPER FUNCTIONS FOR RENDERBASE CREATION
// ========================================

/// Selects the best physical device based on the available hardware, returning the device and the
/// indices of the necessary queues
pub(crate) fn select_physical_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, Vec<u32>) {
    let (physical_device, queue_families) = instance
        .enumerate_physical_devices()
        .unwrap()
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            find_queue_families(
                &[QueueFlags::GRAPHICS, QueueFlags::TRANSFER],
                p.clone(),
                surface,
            )
                .map(|q| (p, q))
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

    println!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type,
    );

    (physical_device, queue_families)
}

// QUEUE FAMILIES

fn find_queue_family(
    required_flags: QueueFlags,
    physical_device: Arc<PhysicalDevice>,
    surface: &Surface,
) -> Option<usize> {
    physical_device
        .queue_family_properties()
        .iter()
        .enumerate()
        .find(|&q| {
            if required_flags.contains(QueueFlags::GRAPHICS)
                && !physical_device
                .surface_support(q.0 as u32, surface)
                .unwrap_or(false)
            {}
            q.1.queue_flags.contains(required_flags)
        })
        .map(|q| q.0)
}

fn find_queue_families(
    required_flags: &[QueueFlags],
    physical_device: Arc<PhysicalDevice>,
    surface: &Surface,
) -> Option<Vec<u32>> {
    let mut queue_families = Vec::new();
    for flags in required_flags.into_iter() {
        if let Some(family) = find_queue_family(flags.clone(), physical_device.clone(), surface) {
            queue_families.push(family as u32);
        } else {
            return None;
        }
    }
    queue_families.sort();
    queue_families.dedup();
    Some(queue_families)
}

pub struct QueueFamilies {
    graphics: u32,
    transfer: u32,
}

/// Gets the Vulkan instance to use for rendering. May need to be modified based on what extensions
/// are required or what version is used
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

/// Creates the physical device, logical device, and queues that will be needed for rendering
pub(crate) fn get_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
) -> (
    Arc<PhysicalDevice>,
    Arc<Device>,
    impl ExactSizeIterator<Item = Arc<Queue>>,
) {
    // Specify features for the physical device with the relevant extensions
    let enabled_extensions = DeviceExtensions {
        khr_swapchain: true,
        khr_storage_buffer_storage_class: true,
        ..DeviceExtensions::empty()
    };

    let (physical_device, queue_families) =
        select_physical_device(instance, surface, &enabled_extensions);

    let queue_create_infos = queue_families
        .iter()
        .map(|q| QueueCreateInfo {
            queue_family_index: *q,
            ..Default::default()
        })
        .collect();

    // Create a device, which is the software representation of the hardware stored in the physical device
    let (device, queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos,
            enabled_extensions,
            ..Default::default()
        },
    )
        .expect("Unable to create logical device!");

    (physical_device, device, queues)
}

/// Creates a swapchain for the provided surface based on the capabilities of the physical device
pub(crate) fn get_swapchain(
    physical_device: &Arc<PhysicalDevice>,
    device: &Arc<Device>,
    surface: &Arc<Surface>,
    window: &Arc<Window>,
) -> (Arc<Swapchain>, Vec<Arc<SwapchainImage>>) {
    let caps = physical_device
        .surface_capabilities(&surface, Default::default())
        .unwrap();
    let usage = caps.supported_usage_flags;
    let image_format = Some(
        physical_device
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0,
    );
    Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: caps.min_image_count, // TODO: +1?
            image_format,
            image_extent: window.inner_size().into(),
            image_usage: usage,
            ..Default::default()
        },
    )
        .unwrap()
}
