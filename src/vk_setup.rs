use vulkano;

use vulkano::buffer::{CpuAccessibleBuffer, TypedBufferAccess};
use vulkano::command_buffer::allocator::{CommandBufferAllocator};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents, RenderPassBeginInfo, PrimaryCommandBufferAbstract};
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage, AttachmentImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::memory::allocator::MemoryAllocator;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{Surface, Swapchain, SwapchainCreateInfo};
use vulkano::Version;
use vulkano::format::{ClearValue, Format};
use winit::window::Window;

use std::sync::Arc;

use crate::geometry::Vertex;

/// Selects the best physical device based on the available hardware
pub fn select_physical_device(
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
pub fn window_size_dependent_setup(
    allocator: &(impl MemoryAllocator + ?Sized),
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> (
    Vec<Arc<Framebuffer>>,
    Arc<ImageView<AttachmentImage>>,
    Arc<ImageView<AttachmentImage>>,
) {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(allocator, dimensions, Format::D16_UNORM).unwrap()
    ).unwrap();
    let color_buffer = ImageView::new_default(
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
    
    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        view, 
                        color_buffer.clone(),
                        normal_buffer.clone(),
                        depth_buffer.clone()
                    ],
                    ..Default::default()
                }
            ).unwrap()
        }).collect::<Vec<_>>();

    (framebuffers, color_buffer.clone(), normal_buffer.clone())
}

pub fn get_instance() -> Arc<Instance> {
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

pub fn get_device(instance: &Arc<Instance>, surface: &Arc<Surface>) -> (Arc<PhysicalDevice>, Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>) {
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

pub fn get_swapchain(
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

pub fn get_render_pass(device: &Arc<Device>, final_format: Format) -> Arc<RenderPass> {
    vulkano::ordered_passes_renderpass!(
        device.clone(),
        attachments: {
            final_color: {
                load: Clear,
                store: Store,
                format: final_format,
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
    .unwrap()
}

pub struct GraphicsInfo<'a> {
    pub vertex_buf: &'a Arc<CpuAccessibleBuffer<[Vertex]>>,
    pub deferred_pipeline: &'a Arc<GraphicsPipeline>,
    pub lighting_pipeline: &'a Arc<GraphicsPipeline>,
    pub deferred_set: &'a Arc<PersistentDescriptorSet>,
    pub lighting_set: &'a Arc<PersistentDescriptorSet>,
}

pub fn get_command_buffer(
    command_buffer_allocator: &impl CommandBufferAllocator,
    queue_family_index: u32,
    clear_values: Vec<Option<ClearValue>>,
    framebuffer_target: Arc<Framebuffer>,
    viewport: &Viewport,
    graphics_info: GraphicsInfo,
) -> impl PrimaryCommandBufferAbstract {
    let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
        command_buffer_allocator,
        queue_family_index,
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    command_buffer_builder
        .begin_render_pass(
            RenderPassBeginInfo { 
                clear_values,
                ..RenderPassBeginInfo::framebuffer(framebuffer_target)
            },
            SubpassContents::Inline,
        )
        .unwrap()
        .set_viewport(0, [viewport.clone()])
        .bind_pipeline_graphics(graphics_info.deferred_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            graphics_info.deferred_pipeline.layout().clone(), 
            0,
            graphics_info.deferred_set.clone()
        )
        .bind_vertex_buffers(0, graphics_info.vertex_buf.clone())
        .draw(graphics_info.vertex_buf.len() as u32, 1, 0, 0)
        .unwrap()
        .next_subpass(SubpassContents::Inline)
        .unwrap()
        .bind_pipeline_graphics(graphics_info.lighting_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Graphics, 
            graphics_info.lighting_pipeline.layout().clone(), 
            0, 
            graphics_info.lighting_set.clone()
        )
        .draw(graphics_info.vertex_buf.len() as u32, 1, 0, 0)
        .unwrap()
        .end_render_pass()
        .unwrap();
    
    command_buffer_builder.build().unwrap()
}