use std::mem::MaybeUninit;
use std::sync::Arc;

use vulkano::buffer::cpu_pool::CpuBufferPoolSubbuffer;
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
use crate::{geometry::MarchedObject, shaders::{ambient_frag, point_frag, albedo_vert}};

use super::{Renderer, RenderBase};

pub struct MarchedRenderer {
    base: RenderBase,

    render_pass: Arc<RenderPass>,

    buffer_allocator: Arc<GenericMemoryAllocator<Arc<FreeListAllocator>>>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,

    ambient_light_buf: Option<Arc<CpuAccessibleBuffer<ambient_frag::ty::UAmbientLightData>>>,
    point_light_buf_pool: CpuBufferPool<marched_frag::ty::UPointLightData>,
    point_light_buf: Arc<CpuBufferPoolSubbuffer<marched_frag::ty::UPointLightData>>,
    // albedo_buf_pool: CpuBufferPool<albedo_vert::ty::Model_Data>,
    vp_buf_pool: CpuBufferPool<albedo_vert::ty::UCamData>,

    vp_set: Option<Arc<PersistentDescriptorSet>>,

    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,

    dummy_vertices: Arc<CpuAccessibleBuffer<[DummyVertex]>>,

    objects: Vec<MarchedObject>,
    ambient_light: AmbientLight,
}

impl MarchedRenderer {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let mut base = RenderBase::new(&event_loop);

        let render_pass = get_render_pass(&base.device, base.swapchain.image_format());
        let shaders = ShaderModulePair::marched_default(&base.device);

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
        let point_light_buf_pool = CpuBufferPool::<marched_frag::ty::UPointLightData>::uniform_buffer(buffer_allocator.clone());
        // let albedo_buf_pool = CpuBufferPool::<albedo_vert::ty::Model_Data>::uniform_buffer(buffer_allocator.clone());
        let vp_buf_pool = CpuBufferPool::<albedo_vert::ty::UCamData>::uniform_buffer(buffer_allocator.clone());

        let point_light_buf = point_light_buf_pool.from_data(
            marched_frag::ty::UPointLightData {
                data: unsafe { get_point_light_arr::<16>(&Vec::new()) },
                len: 0
            }
        ).unwrap();

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
            point_light_buf, 
            // albedo_buf_pool, 
            vp_buf_pool, 

            vp_set: None, 

            render_pass,
            pipeline,
            framebuffers,
            dummy_vertices,

            objects: Vec::new(),
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
    pub fn finish(&mut self) {
        if self.base.render_error { return; }

        // Create the descriptor sets and draw to the scene
        let layout = self.pipeline.layout().set_layouts().get(1).unwrap().clone();
        let lighting_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, self.point_light_buf.clone()), 
                WriteDescriptorSet::buffer(1, self.ambient_light_buf.as_ref().expect("No ambient light added").clone())
            ]
        ).unwrap();
        
        self.base.commands_mut()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics, 
                self.pipeline.layout().clone(), 
                0,
                (self.vp_set.as_ref().unwrap().clone(), lighting_set)
            )
            .bind_vertex_buffers(0, self.dummy_vertices.clone())
            .draw(self.dummy_vertices.len() as u32, 1, 0, 0)
            .unwrap();
        
        self.base.finish();
    }

    pub fn add_object(&mut self, object: &mut MarchedObject) {
        todo!()
    }

    /// Adds point lights to the scene. Unlike in the mesh renderer, these point lights will persist between frames, so there's no need to re-add them unless their positions have been changed. 
    pub fn set_point_lights(&mut self, point_lights: &Vec<PointLight>) {
        self.point_light_buf = self.point_light_buf_pool.from_data(
            marched_frag::ty::UPointLightData {
                data: unsafe { get_point_light_arr::<16>(&point_lights) },
                len: point_lights.len() as i32
            }
        ).unwrap();
    } 

    /// Sets the ambient light to use for rendering
    pub fn set_ambient_light(&mut self, light: AmbientLight) {
        self.ambient_light_buf = Some(CpuAccessibleBuffer::from_data(
            &self.buffer_allocator, 
            BufferUsage {
                uniform_buffer: true,
                ..Default::default()
            }, 
            false, 
            ambient_frag::ty::UAmbientLightData {
                color: light.color.into(),
                intensity: light.intensity.into(),
            },
        ).unwrap())
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

unsafe fn get_point_light_arr<const MAX_LEN: usize>(point_lights: &Vec<PointLight>) -> [marched_frag::ty::UPointLight; MAX_LEN] {
    let mut uninit_array: MaybeUninit<[marched_frag::ty::UPointLight; MAX_LEN]> = MaybeUninit::uninit();
    let mut ptr_i = uninit_array.as_mut_ptr() as *mut marched_frag::ty::UPointLight;

    if point_lights.len() > MAX_LEN { panic!("Only {} point lights may be added to the scene at one time", MAX_LEN) }
    
    for i in 0..point_lights.len() {
        let light = &point_lights[i];
        let position = light.get_position();
        let position_arr = [position.x, position.y, position.z, 0.0];
        let u_light = marched_frag::ty::UPointLight {
            position: position_arr.into(),
            color: *light.get_color(),
            intensity: light.get_intensity(),
        };
        unsafe {
            ptr_i.write(u_light);
            ptr_i = ptr_i.add(1);
        }
    }
    unsafe {
        uninit_array.assume_init()
    }
}