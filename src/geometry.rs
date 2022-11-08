use bytemuck::{Zeroable, Pod};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Vertex {
    position: [f32; 3],
}
vulkano::impl_vertex!(Vertex, position);


// Vertex shader
mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/shaders/shader.vs",
    }
}
// Fragment shader
mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/shaders/shader.fs",
    }
}