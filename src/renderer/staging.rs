use crate::renderer::RenderBase;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo, PrimaryCommandBufferAbstract,
};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryUsage};
use vulkano::sync::GpuFuture;

pub trait StagingBuffer {
    fn into_device_local(
        self,
        buffer_len: u64,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        render_base: &RenderBase,
    ) -> Self;
}

// TODO: remove the buffer_len parameter by improving generics and utilizing len() function of Subbuffer<[T]>

impl<T: BufferContents + ?Sized> StagingBuffer for Subbuffer<T> {
    /// Creates a device local buffer, using this buffer for staging and subsequently executing a
    /// command buffer to copy its contents into the device local buffer on the GPU. This should
    /// only be used for buffers that aren't modified very often, such as vertex buffers.
    ///
    /// The subbuffer that this is called on should have `BufferUsage::TRANSFER_SRC` in its
    /// `buffer_usage`, and `MemoryUsage::Upload` in its `AllocationCreateInfo`. All flags on the
    /// original buffer except for `BufferUsage::TRANSFER_SRC` will be applied to the device-local
    /// buffer, and `BufferUsage::TRANSFER_DST` will automatically be added.
    ///
    /// The value of `buffer_len` should match the length of the buffer in which this function
    /// is called. If this function is called on a non-array type, the value of this parameter
    /// should be set to 1.
    ///
    /// # Panics
    /// - The function will panic if the length passed in through the `buffer_len` parameter is not
    /// equal to the length of the length of the buffer this function is called on.
    fn into_device_local(
        self,
        buffer_len: u64,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        render_base: &RenderBase,
    ) -> Subbuffer<T> {
        let usage = self.buffer().usage().difference(BufferUsage::TRANSFER_SRC);
        let device_local_buf = Buffer::new_unsized::<T>(
            buffer_allocator,
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST | usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                // Specify use for upload to the device.
                usage: MemoryUsage::DeviceOnly,
                ..Default::default()
            },
            buffer_len,
        )
        .unwrap();

        assert_eq!(&self.size(), &device_local_buf.size());

        // Create a one-time command to copy between the buffers.
        let mut cbb = AutoCommandBufferBuilder::primary(
            &render_base.command_buffer_allocator,
            render_base.transfer_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // Add the copy command to the command buffer
        cbb.copy_buffer(CopyBufferInfo::buffers(
            self,
            device_local_buf.clone(), // This is chill because it's basically just cloning an arc (the parent) and a few integers
        ))
        .unwrap();

        // Execute the copy command and wait for completion before proceeding.
        cbb.build()
            .unwrap()
            .execute(render_base.transfer_queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        // println!("Created device-local buffer: {:?}", buffer_usage);

        return device_local_buf;
    }
}

pub(crate) trait UniformSrc {
    type UniformType: BufferContents;
    fn get_raw(&self) -> Self::UniformType;
}

pub(crate) trait IntoPersistentUniform: UniformSrc {
    fn get_current_buffer(&self) -> Option<Subbuffer<Self::UniformType>>;
    fn set_current_buffer(&mut self, buf: Subbuffer<Self::UniformType>);

    fn create_buffer(
        &mut self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        render_base: &RenderBase
    ) -> Subbuffer<Self::UniformType> {
        let buf: Subbuffer<Self::UniformType> = Buffer::from_data(
            buffer_allocator,
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC | BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            self.get_raw()
        )
            .unwrap()
            .into_device_local(1, buffer_allocator, render_base);
        self.set_current_buffer(buf.clone());
        buf
    }

    fn get_buffer(
        &mut self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        render_base: &RenderBase
    ) -> Subbuffer<Self::UniformType>{
        return if let Some(buffer) = self.get_current_buffer().as_ref() {
            buffer.clone()
        } else {
            self.create_buffer(buffer_allocator, render_base)
        }
    }
}
