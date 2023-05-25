use std::mem::MaybeUninit;
use std::sync::Arc;

use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::descriptor_set::WriteDescriptorSet;
use vulkano::descriptor_set::{allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet};
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage};
use vulkano::memory::allocator::{AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, MemoryUsage, StandardMemoryAllocator};
use vulkano::padded::Padded;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};

use crate::geometry::dummy::DummyVertex;
use crate::geometry::marched::Metaball;
use crate::lighting::{AmbientLight, PointLight};
use crate::renderer::staging::{IntoPersistentUniform, StagingBuffer, UniformSrc};
use crate::shaders::{marched_frag, ShaderModulePair};

use crate::UnconfiguredError;

use super::{RenderBase, Renderer};

const MAX_POINT_LIGHTS: usize = 16;
const MAX_METABALLS: usize = 1024;

pub struct MarchedRenderer {
    base: RenderBase,

    render_pass: Arc<RenderPass>,

    buffer_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    subbuffer_allocator: SubbufferAllocator,

    vp_set: Option<Arc<PersistentDescriptorSet>>,
    geometry_set: Option<Arc<PersistentDescriptorSet>>,
    lighting_data: Option<MarchedLightingData>, // Contains a PersistentDescriptorSet and a point light count

    dummy_vertex_buf: Subbuffer<[DummyVertex]>,

    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,

    objects: Vec<Metaball>,
}

impl MarchedRenderer {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let mut base = RenderBase::new(&event_loop);
        let render_pass = get_render_pass(&base.device, base.swapchain.image_format());

        // Buffer allocators
        // Generic allocator for framebuffer attachments, descriptor sets, vertex buffers, etc.
        // TODO: might want to have multiple allocators separated based on function
        let buffer_allocator = Arc::from(
            GenericMemoryAllocator::<Arc<FreeListAllocator>>::new_default(base.device.clone()),
        );
        // TODO: use a descriptor pool instead of a descriptor set allocator
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(base.device.clone());

        let subbuffer_allocator = SubbufferAllocator::new(
            buffer_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                arena_size: 512, // TODO: FIND THE ACTUAL VALUE
                buffer_usage: BufferUsage::UNIFORM_BUFFER,
                memory_usage: MemoryUsage::Upload,
                ..Default::default()
            },
        );

        // Create a dummy vertex buffer used for full-screen shaders
        let dummy_vertex_buf = DummyVertex::buf(&buffer_allocator, &base);

        // Includes framebuffers and other attachments that aren't stored
        let (framebuffers, pipeline) = window_size_dependent_setup(
            &base.images,
            render_pass.clone(),
            &mut base.viewport,
            &base.device,
        );

        Self {
            base,

            render_pass,

            buffer_allocator,
            descriptor_set_allocator,
            subbuffer_allocator,

            vp_set: None,
            geometry_set: None,
            lighting_data: None,

            dummy_vertex_buf,

            pipeline,
            framebuffers,

            objects: vec![],
        }
    }

    /// Starts the rendering process, configuring buffers for the camera, recreating the swapchain
    /// if necessary, and performing necessary acquisition of rendering resources.
    pub fn start(&mut self, camera: &mut crate::camera::Camera) {
        if !camera.is_configured() {
            camera.configure(self.get_window_size());
        }

        let vp_layout = self
            .pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap()
            .clone();
        let vp_subbuffer = camera.get_vp_subbuffer(&self.subbuffer_allocator).unwrap();
        self.vp_set = Some(
            PersistentDescriptorSet::new(
                &self.descriptor_set_allocator,
                vp_layout,
                [WriteDescriptorSet::buffer(0, vp_subbuffer)],
            )
            .unwrap(),
        );

        if self.base.should_recreate_swapchain {
            camera.configure(self.get_window_size());
            self.recreate_all_size_dependent();
        }

        self.base.start(&mut self.framebuffers);
    }

    /// Finishes the rendering process and draws to the screen.
    pub fn finish(&mut self) -> Result<(), UnconfiguredError>{
        if self.base.render_error {
            return Ok(());
        }

        let geometry_set = match self.geometry_set.as_ref() {
            Some(gs) => gs,
            None => { return Err(UnconfiguredError(
                "Geometry descriptor set is not configured! It must be configured with \
                `add_objects()` before the finish() function is called."
            )); }
        }.clone();

        let lighting_data = match self.lighting_data.as_ref() {
            Some(ld) => ld,
            None => { return Err(UnconfiguredError(
                "Lighting descriptor set is not configured! It must be configured with \
                `config_lighting()` before the finish() function is called."
            )); }
        };

        self.base
            .commands_mut()
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                (
                    self.vp_set.as_ref().unwrap().clone(),
                    lighting_data.set.clone(),
                    geometry_set,
                ),
            )
            .bind_vertex_buffers(0, self.dummy_vertex_buf.clone())
            .draw(self.dummy_vertex_buf.len() as u32, 1, 0, 0)
            .unwrap();

        self.base.finish();

        Ok(())
    }

    /// Configures the lighting descriptor set of the scene. Buffers created are device-only, so
    /// this should not be run often.
    pub fn config_lighting(&mut self, point_lights: &mut Vec<PointLight>, ambient_light: &mut AmbientLight) {
        let point_light_count = point_lights.len();

        let point_light_data = unsafe {
            to_partially_init_arr::<MAX_POINT_LIGHTS, Padded<marched_frag::UPointLight, 12>>(
                point_lights.iter()
                    .map(|pl| {
                        let raw: marched_frag::UPointLight = pl.get_raw().into();
                        Padded::from(raw)
                    })
            )
        };

        let point_light_buf: Subbuffer<marched_frag::UPointLightsData> = Buffer::from_data(
            &self.buffer_allocator,
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC | BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            marched_frag::UPointLightsData {
                data: point_light_data,
                len: point_light_count as u32,
            }
        )
            .unwrap()
            .into_device_local(1, &self.buffer_allocator, self.get_base());


        let layout = self.pipeline.layout().set_layouts().get(1).unwrap().clone();
        let set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [
                WriteDescriptorSet::buffer(
                    0,
                    point_light_buf
                ),
                WriteDescriptorSet::buffer(
                    1,
                    ambient_light.get_buffer(&self.buffer_allocator, self.get_base()),
                ),
            ],
        ).unwrap();

        self.lighting_data = Some(MarchedLightingData {
            point_light_count,
            set,
        });
    }

    /// Adds metaball objects to the scene. Metaball objects do not persist between frames, so
    /// this function must be called on a per-frame basis.
    pub fn add_objects(&mut self, objects: &Vec<Metaball>) {
        let objects: Vec<Padded<marched_frag::UMetaball, 12>> = objects
            .iter()
            .map(|obj| {
                Padded::from(obj.get_raw())
            })
            .collect();

        let len = objects.len() as u32;
        let data = unsafe {
            to_partially_init_arr::<MAX_METABALLS, Padded<marched_frag::UMetaball, 12>>(objects)
        };

        let metaball_buf = self.subbuffer_allocator.allocate_unsized(MAX_METABALLS as u64).unwrap();
        *metaball_buf.write().unwrap() = marched_frag::UMetaballData { data, len };

        let layout = self.pipeline.layout().set_layouts().get(2).unwrap().clone();
        self.geometry_set = Some(PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            layout.clone(),
            [WriteDescriptorSet::buffer(0, metaball_buf.clone())],
        ).expect("Unable to create geometry descriptor set"));
    }
}

impl Renderer for MarchedRenderer {
    /// Recreates the ray_marching renderer's framebuffers, pipeline, and swapchain, all of which depend
    /// on the window size.
    fn recreate_all_size_dependent(&mut self) {
        self.base.recreate_swapchain();
        // TODO: use a different allocator?
        let (framebuffers, pipeline) = window_size_dependent_setup(
            &self.base.images,
            self.render_pass.clone(),
            &mut self.base.viewport,
            &self.base.device,
        );
        self.framebuffers = framebuffers;
        self.pipeline = pipeline;
    }
    fn get_base(&self) -> &RenderBase {
        &self.base
    }
}

/// Sets up the framebuffers and graphics pipeline based on the size of the viewport
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
    device: &Arc<Device>,
) -> (Vec<Arc<Framebuffer>>, Arc<GraphicsPipeline>) {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    let pipeline = get_pipeline(&render_pass, dimensions, device);

    (framebuffers, pipeline)
}

/// Gets the render pass to use with the ray_marching renderer. In Vulkan, a render pass is the set of
/// attachments, the way they are used, and the rendering work that is performed using them.
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
    )
    .unwrap()
}

/// Gets the graphics pipeline containing the ray_marching vertex and fragment shaders.
pub fn get_pipeline(
    render_pass: &Arc<RenderPass>,
    dimensions: [u32; 2],
    device: &Arc<Device>,
) -> Arc<GraphicsPipeline> {
    let shaders = ShaderModulePair::marched_default(device);

    GraphicsPipeline::start()
        .vertex_input_state(DummyVertex::per_vertex())
        .vertex_shader(shaders.vert.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
            Viewport {
                origin: [0.0, 0.0],
                dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                depth_range: 0.0..1.0,
            },
        ]))
        .fragment_shader(shaders.frag.entry_point("main").unwrap(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap()
}

struct MarchedLightingData {
    point_light_count: usize,
    set: Arc<PersistentDescriptorSet>,
}

/// A helper function that creates a partially uninitialized array, useful for creating a variable
/// number of objects when a fixed-length array is required.
///
/// # Panics
/// - Panics if the number of elements in `values` exceeds `MAX_LEN`
pub unsafe fn to_partially_init_arr<const MAX_LEN: usize, T>(values: impl IntoIterator<Item = T>) -> [T; MAX_LEN] {
    let mut uninit_array: MaybeUninit<[T; MAX_LEN]> = MaybeUninit::uninit();
    let mut ptr_i = uninit_array.as_mut_ptr() as *mut T;

    for (i, val) in values.into_iter().enumerate() {
        if i + 1 == MAX_LEN {
            panic!(
                "Overflowed maximum capacity of partially initialized array: {}",
                MAX_LEN
            )
        }
        ptr_i.write(val);
        ptr_i = ptr_i.add(1);
    }
    uninit_array.assume_init()
}
