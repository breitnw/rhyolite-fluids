use nalgebra_glm::Vec3;

    /// A metaball, or a sphere that blends with other spheres. The default object in marched rendering. 
    pub struct Metaball {
        position: Vec3,
        color: Vec3,
        radius: f32,
    }    

    impl Metaball {
        pub fn new(position: Vec3, color: Vec3, radius: f32) -> Self {
            Self { position, color, radius }
        }
        pub fn set_position(&mut self, pos: Vec3) {
            self.position = pos;
        }
        pub fn get_position(&self) -> &Vec3 {
            &self.position
        }
        pub fn get_color(&self) -> &Vec3 {
            &self.color
        }
        pub fn get_radius(&self) -> f32 {
            self.radius
        }
    }