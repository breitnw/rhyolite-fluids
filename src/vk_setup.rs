use vulkano;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage, TypedBufferAccess, CpuBufferPool};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents, RenderPassBeginInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage, AttachmentImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::library::VulkanLibrary;
use vulkano::memory::allocator::{GenericMemoryAllocator, FreeListAllocator, MemoryAllocator};
use vulkano::pipeline::graphics::rasterization::{RasterizationState, CullMode};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    self, AcquireError, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
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