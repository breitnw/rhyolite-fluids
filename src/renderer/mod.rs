use vulkano;

use vulkano::buffer::{CpuAccessibleBuffer, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAlloc};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage, AttachmentImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::memory::allocator::{MemoryAllocator, GenericMemoryAllocator, FreeListAllocator};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainAcquireFuture};
use vulkano::Version;
use vulkano::format::{Format};
use vulkano::sync::GpuFuture;
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::event_loop::{self, EventLoop};
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;

use crate::UnconfiguredError;
use crate::camera::Camera;
use crate::geometry::MeshObject;
use crate::lighting::{AmbientLight, PointLight};
use crate::shaders::point_frag;

pub mod mesh;
pub mod marched;

pub trait Renderer {
    type Object;
    fn draw_object(&mut self, object: &mut Self::Object) -> Result<(), UnconfiguredError>;
    fn set_ambient(&mut self, light: AmbientLight);
    fn draw_ambient_light(&mut self);
    fn draw_point_light(&mut self, camera: &mut Camera, point_light: &mut PointLight);
    fn draw_object_unlit(&mut self, object: &mut Self::Object) -> Result<(), UnconfiguredError>;
    fn recreate_swapchain(&mut self);
}

pub struct BaseRenderer {
    instance: Arc<Instance>,
    surface: Arc<Surface>,
    window: Arc<Window>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,

    buffer_allocator: Arc<GenericMemoryAllocator<Arc<FreeListAllocator>>>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    command_buffer_allocator: StandardCommandBufferAllocator,

    framebuffers: Vec<Arc<Framebuffer>>,
    attachment_buffers: AttachmentBuffers,

    viewport: Viewport,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>, StandardCommandBufferAllocator>>,
    image_idx: u32,
    acquire_future: Option<SwapchainAcquireFuture>,
    should_recreate_swapchain: bool,
}

impl BaseRenderer {
    fn new(event_loop: &EventLoop<()>) -> Self { 
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

        

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        // Includes framebuffers and other attachments that aren't stored
        let (framebuffers, attachment_buffers) = window_size_dependent_setup(&buffer_allocator, &images, render_pass.clone(), &mut viewport);

        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        let commands = None;
        let image_idx = 0;
        let acquire_future = None;

        let render_stage = RenderStage::Stopped;

        Self{ 
            instance, 
            surface, 
            window,
            device, 
            queue,  
            swapchain,

            framebuffers,
            attachment_buffers,

            viewport,
            previous_frame_end,

            commands, 
            image_idx,
            acquire_future,

            should_recreate_swapchain: false,
        } 
    }

    /// Updates the aspect ratio of the camera. Should be called when the window is resized
    pub fn update_aspect_ratio(&mut self, camera: &mut Camera) {
        camera.configure(&self.window);
    }

    /// Sets up necessary buffers and attaches them to the object
    pub fn configure_object(&self, object: &mut MeshObject) {
        object.configure(&self.buffer_allocator)
    }
}

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


pub(crate) struct AttachmentBuffers {
    pub albedo_buffer: Arc<ImageView<AttachmentImage>>,
    pub normal_buffer: Arc<ImageView<AttachmentImage>>,
    pub frag_pos_buffer: Arc<ImageView<AttachmentImage>>,
    pub specular_buffer: Arc<ImageView<AttachmentImage>>,
}

/// Sets up the framebuffers based on the size of the viewport.
pub(crate) fn window_size_dependent_setup(
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

pub(crate) fn get_render_pass(device: &Arc<Device>, final_format: Format) -> Arc<RenderPass> {
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