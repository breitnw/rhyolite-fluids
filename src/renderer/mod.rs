use vulkano;

use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage, AttachmentImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::memory::allocator::MemoryAllocator;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use vulkano::Version;
use vulkano::format::{Format};
use winit::event_loop::{self, EventLoop};
use winit::window::Window;

use std::sync::Arc;

use crate::UnconfiguredError;
use crate::camera::Camera;
use crate::lighting::{AmbientLight, PointLight};

pub mod mesh;
pub mod marched;

pub trait Renderer {
    type Object;
    fn new(event_loop: &EventLoop<()>) -> Self;
    fn draw_object(&mut self, object: &mut Self::Object) -> Result<(), UnconfiguredError>;
    fn set_ambient(&mut self, light: AmbientLight);
    fn draw_ambient_light(&mut self);
    fn draw_point_light(&mut self, camera: &mut Camera, point_light: &mut PointLight);
    fn draw_object_unlit(&mut self, object: &mut Self::Object) -> Result<(), UnconfiguredError>;
    fn recreate_swapchain(&mut self);
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