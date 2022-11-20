use bytemuck::{Zeroable, Pod};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct DummyVertex {
    pub position: [f32; 2]
}

impl DummyVertex {
    pub fn list() -> [DummyVertex; 6] {
        [
            DummyVertex { position: [-1.0, -1.0] },
            DummyVertex { position: [-1.0, 1.0] },
            DummyVertex { position: [1.0, 1.0] },
            DummyVertex { position: [-1.0, -1.0] },
            DummyVertex { position: [1.0, 1.0] },
            DummyVertex { position: [1.0, -1.0] }
        ]
    }
}
