use std::cell::Cell;
use nalgebra_glm::{perspective, TMat4};
use vulkano::buffer::allocator::SubbufferAllocator;
use vulkano::buffer::Subbuffer;

use crate::{shaders::albedo_vert, transform::Transform, UnconfiguredError};
use crate::renderer::staging::UniformSrc;

pub struct Camera {
    fovy: f32,
    near_clipping_plane: f32,
    far_clipping_plane: f32,
    transform: Transform,
    /// A struct containing post-configuration data that must be added after the engine is created
    post_config: Option<CameraPostConfig>,
    /// A cache of the view matrix so that it doesn't need to be recalculated every frame
    cache: Cell<Option<TMat4<f32>>>,
}

struct CameraPostConfig {
    aspect_ratio: f32,
    projection: TMat4<f32>,
} // :)

impl Camera {
    /// Creates a new camera with a specified transform, FOV, and clipping planes.
    /// * `transform`: The transform to create the camera with, ignoring scale.
    /// * `fovy`: The camera's vertical field of view.
    /// * `near_clipping_plane`: The nearest distance at which geometry will clip out of view.
    /// * `far_clipping_plane`: The farthest distance at which geometry will clip out of view.
    pub fn new(
        transform: Transform,
        fovy: f32,
        near_clipping_plane: f32,
        far_clipping_plane: f32,
    ) -> Self {
        Camera {
            transform,
            fovy,
            near_clipping_plane,
            far_clipping_plane,
            post_config: None,
            cache: Cell::new(None),
        }
    }

    // TODO: find a way to do this automatically
    /// Configures the camera's aspect ratio. Needs to be run before the camera can be used.
    pub fn configure(&mut self, dimensions: [i32; 2]) {
        let aspect_ratio = dimensions[0] as f32 / dimensions[1] as f32;
        let projection = perspective(
            aspect_ratio,
            self.fovy,
            self.near_clipping_plane,
            self.far_clipping_plane,
        );

        self.post_config = Some(CameraPostConfig {
            aspect_ratio,
            projection,
        });
    }

    fn get_post_config(&self) -> Result<&CameraPostConfig, UnconfiguredError> {
        self.post_config.as_ref().ok_or(UnconfiguredError(
            "Camera not yet configured. Do so with `Camera::configure()` before accessing projection matrix"
        ))
    }

    fn get_post_config_mut(&mut self) -> Result<&mut CameraPostConfig, UnconfiguredError> {
        self.post_config.as_mut().ok_or(UnconfiguredError(
            "Camera not yet configured. Do so with `Camera::configure()` before accessing projection matrix"
        ))
    }

    pub(crate) fn is_configured(&self) -> bool {
        self.post_config.is_some()
    }

    /// Gets a mutable reference to the camera's transform.
    ///
    /// Calling this function forces the camera's subbuffers to be updated at the end of the frame,
    /// so only use it when it's necessary to move the camera.
    pub fn transform_mut(&mut self) -> &mut Transform {
        self.cache.set(None);
        &mut self.transform
    }

    /// Gets an immutable reference to the camera's transform.
    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    /// Calculates the camera's view matrix, and caches the result
    fn get_view(&self) -> TMat4<f32> {
        if let Some(cache) = self.cache.get() {
            return cache;
        }
        let view = self
            .transform
            .get_matrices()
            .0
            .try_inverse()
            .unwrap();
        self.cache.set(Some(view));
        view
    }

    /// Returns a subbuffer containing the camera's view and projection data as required for
    /// rendering. Allocates from a `SubbufferAllocator`.
    pub(crate) fn get_vp_subbuffer(
        &mut self,
        subbuffer_allocator: &SubbufferAllocator,
    ) -> Result<Subbuffer<albedo_vert::UCamData>, UnconfiguredError> {
        let buf = subbuffer_allocator.allocate_sized().unwrap();
        *buf.write().unwrap() = self.get_raw();

        Ok(buf)
    }
}

impl UniformSrc<albedo_vert::UCamData> for Camera {
    fn get_raw(&self) -> albedo_vert::UCamData {
        albedo_vert::UCamData {
            view: self.get_view().into(),
            projection: self.get_post_config().expect( // TODO: EWWW EXPECT (should be unconfigurederror somehow)
                "Camera must be configured before getting buffer"
            ).projection.into(),
        }
    }
}