use std::sync::Arc;

use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::{SwapchainImage, ImageAccess};
use vulkano::image::view::ImageView;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::render_pass::{RenderPass, Framebuffer, FramebufferCreateInfo, Subpass};
use vulkano::memory::allocator::{GenericMemoryAllocator, FreeListAllocator};
use vulkano::descriptor_set::{allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet};
use vulkano::buffer::{CpuAccessibleBuffer, CpuBufferPool, BufferUsage, TypedBufferAccess};
use vulkano::pipeline::{GraphicsPipeline, PipelineBindPoint, Pipeline};

use crate::camera::Camera;
use crate::geometry::dummy::DummyVertex;
use crate::lighting::{PointLight, AmbientLight};
use crate::shaders::{ShaderModulePair, marched_vert, marched_frag};
use crate::{geometry::MarchedObject, shaders::{ambient_frag, point_frag, albedo_vert, unlit_vert}};

use super::{Renderer, RenderBase};

pub struct MarchedRenderer {
    base: RenderBase,

    render_pass: Arc<RenderPass>,

    buffer_allocator: Arc<GenericMemoryAllocator<Arc<FreeListAllocator>>>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,

    ambient_light_buf: Option<Arc<CpuAccessibleBuffer<ambient_frag::ty::Ambient_Light_Data>>>,
    point_light_buf_pool: CpuBufferPool<point_frag::ty::Point_Light_Data>,
    albedo_buf_pool: CpuBufferPool<albedo_vert::ty::Model_Data>,
    vp_buf_pool: CpuBufferPool<albedo_vert::ty::VP_Data>,

    vp_set: Option<Arc<PersistentDescriptorSet>>,

    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,

    dummy_vertices: Arc<CpuAccessibleBuffer<[DummyVertex]>>,

    objects: Vec<MarchedObject>,
    point_lights: Vec<PointLight>,
    ambient_light: AmbientLight,
}

impl MarchedRenderer {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let mut base = RenderBase::new(&event_loop);

        let render_pass = get_render_pass(&base.device, base.swapchain.image_format());
        let shaders = ShaderModulePair {
            vert: marched_vert::load(base.device.clone()).unwrap(),
            frag: marched_frag::load(base.device.clone()).unwrap(),
        };

        // Render pipelines
        let pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<DummyVertex>())
            .vertex_shader(shaders.vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant()) // TODO: this could probably be fixed_scissor_irrelevant
            .fragment_shader(shaders.frag.entry_point("main").unwrap(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(base.device.clone())
            .unwrap();

        // Buffer allocators
        // Generic allocator for framebuffer attachments, descriptor sets, vertex buffers, etc. 
        // TODO: might want to have multiple allocators separated based on function
        let buffer_allocator = Arc::from(GenericMemoryAllocator::<Arc<FreeListAllocator>>::new_default(base.device.clone()));
        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(base.device.clone());

        // Buffers and buffer pools
        let ambient_light_buf = None;
        let point_light_buf_pool = CpuBufferPool::<point_frag::ty::Point_Light_Data>::uniform_buffer(buffer_allocator.clone());
        let albedo_buf_pool = CpuBufferPool::<albedo_vert::ty::Model_Data>::uniform_buffer(buffer_allocator.clone());
        let vp_buf_pool = CpuBufferPool::<albedo_vert::ty::VP_Data>::uniform_buffer(buffer_allocator.clone());

        // Includes framebuffers and other attachments that aren't stored
        let framebuffers = window_size_dependent_setup(
            &base.images, 
            render_pass.clone(), 
            &mut base.viewport
        );

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

        let ambient_light = AmbientLight {
            color: [1.0, 1.0, 1.0],
            intensity: 0.4, 
        };

        Self { 
            base, 

            buffer_allocator,
            descriptor_set_allocator,

            ambient_light_buf, 
            point_light_buf_pool, 
            albedo_buf_pool, 
            vp_buf_pool, 

            vp_set: None, 

            render_pass,
            pipeline,
            framebuffers,
            dummy_vertices,

            objects: Vec::new(),
            point_lights: Vec::new(),
            ambient_light,
        }
    }

    pub fn start(&mut self, camera: &mut crate::camera::Camera) {
        let base = &self.base;

        if !camera.is_configured() {
            camera.configure(&base.window);
        }

        let vp_layout = self.pipeline.layout().set_layouts().get(0).unwrap().clone();
        let vp_subbuffer = camera.get_vp_subbuffer(&self.vp_buf_pool).unwrap();
        self.vp_set = Some(PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            vp_layout,
            [
                WriteDescriptorSet::buffer(0, vp_subbuffer),
            ]
        ).unwrap());

        if self.base.should_recreate_swapchain {
            camera.configure(&base.window);
            self.recreate_swapchain_and_buffers();
        }

        self.base.start(&mut self.framebuffers);
    }

    /// Finishes the rendering process and draws to the screen
    pub fn finish(&mut self, time: f32) {
        if self.base.render_error { return; }

        // TODO: BAD!!!!!!!!!!!!!!!!!!!!!!!!!
        let buf = CpuAccessibleBuffer::from_data(
            &self.buffer_allocator, 
            BufferUsage { 
                uniform_buffer: true,
                ..Default::default()
            }, 
            false,
            marched_frag::ty::marching_data {
                time,
            },
        ).unwrap();
        let layout = self.pipeline.layout().set_layouts().get(1).unwrap().clone();
        let set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [WriteDescriptorSet::buffer(0, buf)]
        ).unwrap();
        
        self.base.commands_mut()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.pipeline.layout().clone(), 
                0,
                (self.vp_set.as_ref().unwrap().clone(), set)
            )
            .bind_vertex_buffers(0, self.dummy_vertices.clone())
            .draw(self.dummy_vertices.len() as u32, 1, 0, 0)
            .unwrap();
        
        self.base.finish();
    }

    fn add_object(&mut self, object: &mut MarchedObject) {

    }

    fn add_point_light(&mut self, point_light: &mut PointLight) {

    } 

    fn set_ambient(&mut self, ambient_light: AmbientLight) {

    }

    /// Updates the aspect ratio of the camera. Should be called when the window is resized
    pub fn update_aspect_ratio(&mut self, camera: &mut Camera) {
        camera.configure(&self.base.window);
    }
}

impl Renderer for MarchedRenderer {
    type Object = MarchedObject;

    fn recreate_swapchain_and_buffers(&mut self) {
        self.base.recreate_swapchain();
        // TODO: use a different allocator?
        self.framebuffers = window_size_dependent_setup(
            &self.base.images, 
            self.render_pass.clone(), 
            &mut self.base.viewport
        );
    }
}

/// Sets up the framebuffers based on the size of the viewport.
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
            ).unwrap()
        }).collect::<Vec<_>>()
}

pub(crate) fn get_render_pass(device: &Arc<Device>, final_format: Format) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            final_color: {
                load: Clear,
                store: Store,
                format: final_format,
                samples: 1,
            }
        },
        pass: {
            color: [final_color],
            depth_stencil: {}
        }
    ).unwrap()
}