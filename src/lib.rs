#![allow(dead_code)]

mod renderer;

use std::{time::Instant, collections::HashSet};

use camera::Camera;
use geometry::{loader::ModelBuilder, Object};
use lighting::{AmbientLight, DirectionalLight};
use nalgebra_glm::{translate, identity, vec3, rotate_y, rotate_x, rotate_z, scale};
pub use renderer::Renderer;
use transform::Transform;
use winit::{event_loop::{EventLoop, ControlFlow}, event::{Event, WindowEvent, VirtualKeyCode, ElementState}};

mod shaders;
mod vk_setup;
mod transform;
mod geometry;
mod lighting;
mod camera;


// TODO: implement frames in flight if not implemented in the tutorial

// TODO: fix point lights to use a position buffer for relative distance instead of normal buffer

pub struct Rhyolite {
    renderer: Renderer,
}
impl Rhyolite {
    pub fn run() {
        let camera_transform = Transform::zero();
        let mut camera = Camera::new(camera_transform, 1.2, 0.02, 100.0);

        let event_loop = EventLoop::new();
        let mut renderer = Renderer::new(&event_loop);

        // Build the models
        let mut suzanne = {
            let vertices = ModelBuilder::from_file("data/models/monkey_smooth.obj", true).build_with_color([1.0, 1.0, 1.0]);
            let mut object_transform = Transform::zero();
            object_transform.set_translation_mat(translate(&identity(), &vec3(-1.0, -2.0, -5.0)));
            Object::new(object_transform, vertices)
        };
        suzanne.configure(&renderer.buffer_allocator);

        let mut teapot = {
            let vertices = ModelBuilder::from_file("data/models/teapot.obj", true).build_with_color([1.0, 1.0, 1.0]);
            let mut object_transform = Transform::zero();
            object_transform.set_translation_mat(translate(&identity(), &vec3(3.0, 2.0, -10.0)));
            object_transform.set_scale_mat(scale(&identity(), &vec3(0.5, 0.5, 0.5)));
            Object::new(object_transform, vertices)
        };
        teapot.configure(&renderer.buffer_allocator);

        // Lighting
        let ambient_light = AmbientLight {
            color: [1.0, 1.0, 1.0],
            intensity: 0.2
        };
        renderer.set_ambient(ambient_light);
        let directional_lights = vec![
            DirectionalLight {position: [-4.0, 0.0, -2.0, 1.0], color: [1.0, 0.0, 0.0]},
            DirectionalLight {position: [0.0, -4.0, 1.0, 1.0], color: [0.0, 1.0, 0.0]},
            DirectionalLight {position: [4.0, -2.0, -1.0, 1.0], color: [0.0, 0.0, 1.0]},
            // DirectionalLight {position: [0.0, -4.0, -1.0, 1.0], color: [0.0, 1.0, 0.0]},

        ];

        // Time
        let mut t: f32 = 0.0;
        let time_start = Instant::now();

        // Input
        let mut pressed_keys: HashSet<VirtualKeyCode> = HashSet::new();
        let mut held_keys: HashSet<VirtualKeyCode> = HashSet::new();
        let mut released_keys: HashSet<VirtualKeyCode> = HashSet::new();

        // Other
        let mut camera_pos = vec3(0.0, 0.0, 0.0);
        let mut camera_euler = vec3(0.0, 0.0, 0.0);

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event: WindowEvent::KeyboardInput { input, .. }, .. } => {
                    let keycode = input.virtual_keycode.unwrap();
                    if input.state == ElementState::Pressed {
                        // Check if the key is already held before inserting to prevent key repeats from registering
                        if !held_keys.contains(&keycode) {
                            pressed_keys.insert(keycode);
                            held_keys.insert(keycode);
                        }
                    } else {
                        held_keys.remove(&input.virtual_keycode.unwrap());
                        released_keys.insert(keycode);
                    }
                }
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                    renderer.recreate_swapchain();
                    renderer.update_aspect_ratio(&mut camera);
                },
                Event::RedrawEventsCleared => {
                    // Update time-related variables
                    let prev_t = t;
                    t = time_start.elapsed().as_secs_f32();
                    let delta = t - prev_t;

                    suzanne.transform.set_translation_mat(translate(&identity(), &vec3(t.cos() - 1.0,  - 0.5, -5.0)));
                    // suzanne.transform.set_translation_mat(translate(&identity(), &vec3(t.cos() * 5.0,  - 2.0, -5.0)));
                    suzanne.transform.set_rotation_mat({
                        let mut rotation = identity();
                        rotation = rotate_y(&rotation, t);
                        rotation = rotate_x(&rotation, t / 2.);
                        rotation = rotate_z(&rotation, t / 3.);
                        rotation
                    });

                    teapot.transform.set_rotation_mat({
                        let mut rotation = identity();
                        rotation = rotate_x(&rotation, -t);
                        rotation = rotate_y(&rotation, t / 5.0);
                        rotation = rotate_z(&rotation, t / 2.);
                        rotation
                    });
                    
                    // Camera movement
                    const CAM_MOVE_SPEED: f32 = 4.0;
                    const CAM_ROT_SPEED: f32 = 1.0;

                    let camera_move = vec3(
                        if held_keys.contains(&VirtualKeyCode::D) { CAM_MOVE_SPEED } else { 0.0 } - if held_keys.contains(&VirtualKeyCode::A) { CAM_MOVE_SPEED } else { 0.0 },
                        if held_keys.contains(&VirtualKeyCode::LShift) { CAM_MOVE_SPEED } else { 0.0 } - if held_keys.contains(&VirtualKeyCode::Space) { CAM_MOVE_SPEED } else { 0.0 },
                        if held_keys.contains(&VirtualKeyCode::S) { CAM_MOVE_SPEED } else { 0.0 } - if held_keys.contains(&VirtualKeyCode::W) { CAM_MOVE_SPEED } else { 0.0 },
                    );
                    let camera_rotate = vec3(
                        if held_keys.contains(&VirtualKeyCode::Down) { CAM_ROT_SPEED } else { 0.0 } 
                            - if held_keys.contains(&VirtualKeyCode::Up) { CAM_ROT_SPEED } else { 0.0 },
                        if held_keys.contains(&VirtualKeyCode::Left) { CAM_ROT_SPEED } else { 0.0 } 
                            - if held_keys.contains(&VirtualKeyCode::Right) { CAM_ROT_SPEED } else { 0.0 },
                        0.0,
                    );
                    if !(camera_move.is_empty() && camera_rotate.is_empty()) {
                        camera_euler += camera_rotate * delta;
                        camera_euler.x = camera_euler.x.clamp(-3.1415 / 2.0, 3.1415 / 2.0);
                        camera_pos += nalgebra_glm::rotate_y_vec3(&camera_move, camera_euler.y) * delta;

                        let transform = camera.transform_mut();
                        transform.set_rotation_mat(rotate_z(&rotate_x(&rotate_y(&identity(), camera_euler.y), camera_euler.x), camera_euler.z));
                        transform.set_translation_mat(translate(&identity(), &camera_pos));
                        
                    }
                    
                    renderer.start(&mut camera);
                    renderer.draw_object(&mut suzanne).unwrap();
                    renderer.draw_object(&mut teapot).unwrap();
                    renderer.draw_ambient();
                    for directional_light in &directional_lights {
                        renderer.draw_directional(directional_light);
                    }
                    renderer.finish();
                },
                _ => ()
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct UnconfiguredError(&'static str);
impl std::fmt::Display for UnconfiguredError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}