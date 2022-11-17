#[derive(Default, Debug, Clone)]
pub struct AmbientLight {
    pub color: [f32; 3],
    pub intensity: f32,
}

#[derive(Default, Debug, Clone)]
pub struct DirectionalLight {
    pub position: [f32; 4],
    pub color: [f32; 3]
}
