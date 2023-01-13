use vulkano::swapchain::SwapchainCreateInfo;

use crate::geometry::MarchedObject;

use super::Renderer;

pub struct MarchedRenderer {

}

impl Renderer for MarchedRenderer {
    type Object = MarchedObject;

    fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        todo!()
    }

    fn draw_object(&mut self, object: &mut Self::Object) -> Result<(), crate::UnconfiguredError> {
        todo!()
    }

    fn set_ambient(&mut self, light: crate::lighting::AmbientLight) {
        todo!()
    }

    fn draw_ambient_light(&mut self) {
        todo!()
    }

    fn draw_point_light(&mut self, camera: &mut crate::camera::Camera, point_light: &mut crate::lighting::PointLight) {
        todo!()
    }

    fn draw_object_unlit(&mut self, object: &mut Self::Object) -> Result<(), crate::UnconfiguredError> {
        todo!()
    }

    /// Recreates the swapchain. Should be called if the swapchain is invalidated, such as by a window resize
    fn recreate_swapchain(&mut self) {
        // let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
        //     image_extent: self.window.inner_size().into(),
        //     ..self.swapchain.create_info()
        // }) {
        //     Ok(r) => r,
        //     Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
        //     Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        // };

        // self.swapchain = new_swapchain;
        // // TODO: use a different allocator?
        // (self.framebuffers, self.attachment_buffers) = super::window_size_dependent_setup(&self.buffer_allocator, &new_images, self.render_pass.clone(), &mut self.viewport);
        todo!()
    }

}