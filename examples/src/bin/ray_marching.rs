
use rhyolite::geometry::marched::Metaball;
use rhyolite::lighting::AmbientLight;
use rhyolite::transform::Transform;
use rhyolite::Rhyolite;
use rhyolite::{camera::Camera, lighting::PointLight};

use winit::event::{Event, VirtualKeyCode, WindowEvent};
use nalgebra_glm::{vec3, Vec3};

use examples::{CamRotationMode, KeyBinding};

use rhyolite::renderer::Renderer;

fn main() {
    let mut rhyolite = Rhyolite::ray_marched();

    let camera_transform = Transform::identity();
    let mut camera = Camera::new(camera_transform, 1.2, 0.02, 100.0);

    // Add point lights to the scene
    let mut point_lights = vec![
        PointLight::new(
            vec3(0.0, 10.0, -10.0),
            vec3(0.0, 0.2, 1.0),
            60.0
        ),
        PointLight::new(
            vec3(0.0, -10.0, 10.0),
            vec3(1.0, 0.2, 0.0),
            60.0
        ),
    ];
    let mut ambient_light = AmbientLight::new(
        vec3(1.0, 1.0, 1.0),
        0.1
    );

    rhyolite.renderer.config_lighting(&mut point_lights, &mut ambient_light);

    const GRID_WIDTH: u32 = 3;
    const GRID_HEIGHT: u32 = 3;

    let mut metaballs: Vec<Metaball> = Vec::with_capacity(81);
    metaballs.push(Metaball::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0), 0.6));
    for i in 0..(GRID_WIDTH * GRID_HEIGHT) {
        metaballs.push(Metaball::new(
            vec3(
                (i / GRID_HEIGHT) as f32 * 2.,
                0.0,
                (i % GRID_HEIGHT) as f32 * 2.,
            ),
            vec3(1.0, 1.0, 1.0),
            0.45,
        ));
    }

    let mut ctrl_metaball_pos = vec3(0.0, 0.0, 0.0);
    let mut control_mode = false;

    // Other
    let mut camera_pos: Vec3 = vec3(0.0, 0.0, 0.0);
    let mut camera_euler: Vec3 = vec3(0.0, 0.0, 0.0);

    rhyolite.run(move |event, keyboard, _, _, time, renderer| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                camera.configure(renderer.get_window_size());
            }
            Event::RedrawEventsCleared => {
                if keyboard.key_pressed(VirtualKeyCode::Escape) {
                    control_mode = !control_mode;
                }

                if !control_mode {
                    examples::do_camera_movement(
                        CamRotationMode::Marched,
                        &mut camera,
                        &mut camera_euler,
                        &mut camera_pos,
                        &keyboard,
                        time.delta,
                    );

                    ctrl_metaball_pos = vec3(2.0, time.current.sin() * 3., 2.0)

                } else {
                    let wasd_move = examples::get_axes(keyboard, KeyBinding::WASD);
                    ctrl_metaball_pos +=
                        nalgebra_glm::rotate_y_vec3(&wasd_move, camera_euler.y) * 0.1;
                }

                metaballs[0].set_position(ctrl_metaball_pos);

                // Rendering
                renderer.start(&mut camera);
                renderer.add_objects(&metaballs);
                renderer.finish().unwrap();
            }
            _ => (),
        }
    });
}
