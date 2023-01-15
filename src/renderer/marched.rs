use crate::geometry::MarchedObject;

use super::Renderer;

pub struct MarchedRenderer {

}

impl MarchedRenderer {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        todo!()
    }
}

impl Renderer for MarchedRenderer {
    type Object = MarchedObject;

    fn start(&mut self, camera: &mut crate::camera::Camera) {
        todo!()
    }

    fn finish(&mut self) {
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

    fn recreate_swapchain_and_buffers(&mut self) {
        todo!()
    }
}