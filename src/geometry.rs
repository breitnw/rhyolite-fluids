use bytemuck::{Zeroable, Pod};
use nalgebra_glm::{TMat4, identity, perspective, look_at, vec3, translate, rotate, rotate_y};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}
impl Vertex {
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            position,
            color,
        }
    }
}
vulkano::impl_vertex!(Vertex, position, color);


// Vertex shader
pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/shader.vs",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

// Fragment shader
pub mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/shader.fs",
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MVP {
    pub model: TMat4<f32>,
    pub view: TMat4<f32>,
    pub projection: TMat4<f32>,
}

impl MVP {
    pub fn new() -> MVP {
        MVP { 
            model: identity(), 
            view: identity(), 
            projection: identity()
        }
    }

    pub fn perspective(aspect_ratio: f32, t: f32) -> MVP {
        MVP { 
            model: rotate_y(&translate(&identity(), &vec3(0.0, t.sin() / 3., -2.0)), t), 
            view: look_at(&vec3(0.0, 0.0, 0.0), &vec3(0.0, 0.0, -0.01), &vec3(0.0, 1.0, 0.0)), 
            projection: perspective(aspect_ratio, 1.2, 0.0, 100.0)
        }
    }
}

