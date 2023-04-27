use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo, PrimaryCommandBufferAbstract,
};
use vulkano::device::Queue;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryUsage};
use vulkano::sync::GpuFuture;

pub(crate) trait StagingBuffer {
    fn into_device_local(
        self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        buffer_usage: BufferUsage,
        command_buf_allocator: &StandardCommandBufferAllocator,
        queue: Arc<Queue>,
    ) -> Self;
}

impl<T: BufferContents + ?Sized> StagingBuffer for Subbuffer<T> {
    /// Creates a device local buffer, using this buffer for staging and subsequently executing a command
    /// buffer to copy its contents into the device local buffer on the GPU. This should only be used for buffers that
    /// aren't modified very often, such as vertex buffers.
    ///
    /// The subbuffer that this is called on should have `BufferUsage::TRANSFER_SRC` in its `buffer_usage`,
    /// and `MemoryUsage::Upload` in its `AllocationCreateInfo`.
    ///
    /// The queue used must be a member of a queue family with the `VK_QUEUE_TRANSFER_BIT`, but not the
    /// `VK_QUEUE_GRAPHICS_BIT`.
    fn into_device_local(
        self,
        buffer_allocator: &(impl MemoryAllocator + ?Sized),
        buffer_usage: BufferUsage,
        command_buf_allocator: &StandardCommandBufferAllocator,
        queue: Arc<Queue>,
    ) -> Subbuffer<T> {

        let size = self.size();
        let device_local_buf = Buffer::new_unsized::<T>(
            buffer_allocator,
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST | buffer_usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                // Specify use for upload to the device.
                usage: MemoryUsage::DeviceOnly,
                ..Default::default()
            },
            size,
        )
        .unwrap();

        // Create a one-time command to copy between the buffers.
        let mut cbb = AutoCommandBufferBuilder::primary(
            command_buf_allocator,
            queue.queue_family_index(),
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
            .execute(queue.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        return device_local_buf;
    }
}
