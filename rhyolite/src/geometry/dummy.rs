use crate::renderer::staging::StagingBuffer;
use crate::renderer::RenderBase;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryUsage};
use vulkano::pipeline::graphics::vertex_input::Vertex;

#[repr(C)]
#[derive(Vertex, Clone, Copy, Debug, BufferContents)]
pub(crate) struct DummyVertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
}

impl DummyVertex {
    pub fn list() -> [DummyVertex; 6] {
        [
            DummyVertex {
                position: [-1.0, -1.0],
            },
            DummyVertex {
                position: [-1.0, 1.0],
            },
            DummyVertex {
                position: [1.0, 1.0],
            },
            DummyVertex {
                position: [-1.0, -1.0],
            },
            DummyVertex {
                position: [1.0, 1.0],
            },
            DummyVertex {
                position: [1.0, -1.0],
            },
        ]
    }

    pub fn buf(
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        base: &RenderBase,
    ) -> Subbuffer<[DummyVertex]> {
        Buffer::from_iter(
            buffer_allocator,
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC | BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            DummyVertex::list(),
        )
        .unwrap()
        .into_device_local(6, buffer_allocator, &base)
    }
}
