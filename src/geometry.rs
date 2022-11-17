use bytemuck::{Zeroable, Pod};
use nalgebra_glm::{TMat4, identity, perspective, look_at, vec3, translate, rotate_y, rotate_x, rotate_z};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

vulkano::impl_vertex!(Vertex, position, normal, color);

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
        let mut model = translate(&identity(), &vec3(0.0, t.sin(), -5.0));
        model = rotate_y(&model, t);
        model = rotate_x(&model, t / 2.);
        model = rotate_z(&model, t / 3.);

        MVP { 
            model,
            view: look_at(&vec3(0.0, 0.0, 0.0), &vec3(0.0, 0.0, -0.01), &vec3(0.0, 1.0, 0.0)), 
            projection: perspective(aspect_ratio, 1.2, 0.02, 100.0)
        }
    }
}