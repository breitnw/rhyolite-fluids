
use rhyolite::camera::Camera;
use rhyolite::geometry::marched::Metaball;
use rhyolite::lighting::{AmbientLight, PointLight};
use rhyolite::renderer::mesh::DrawInfo;
use rhyolite::transform::Transform;
use rhyolite::{Rhyolite, TimeState};

use winit::event::{Event, VirtualKeyCode, WindowEvent};
use nalgebra_glm::{vec3, Vec3, translate, identity};
use examples::{CamRotationMode, KeyBinding};
use rhyolite::geometry::mesh::{MeshObjectParams, BasicVertex};
use rhyolite::renderer::Renderer;

mod marching_cubes;
mod metaball;

use crate::marching_cubes::MarchingCubesGenerator;

fn main() {
    let rhyolite = Rhyolite::mesh();

    let camera_transform = Transform::identity();
    let mut camera = Camera::new(camera_transform, 1.2, 0.02, 100.0);

    let mut generator = MarchingCubesGenerator::new(&rhyolite.renderer);

    // Lighting
    let mut ambient_light = AmbientLight::new(
        vec3(1.0, 1.0, 1.0),
        0.1
    );

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

    // Other
    let mut camera_pos: Vec3 = vec3(0.0, 0.0, 0.0);
    let mut camera_euler: Vec3 = vec3(0.0, 0.0, 0.0);
    
    let params = MeshObjectParams {
        transform: Transform::identity(),
        specular_intensity: 1.0,
        shininess: 64.0,
    };

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

    rhyolite.run(move |event, keyboard, _, _, time, renderer| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                camera.configure(renderer.get_window_size());
                generator.recreate_graphics_pipeline(renderer);
            }
            Event::RedrawEventsCleared => {
                if keyboard.key_pressed(VirtualKeyCode::Escape) {
                    control_mode = !control_mode;
                }

                if !control_mode {
                    examples::do_camera_movement(
                        CamRotationMode::Mesh,
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

                let vertex_buffer = generator.vertex_buffer();
                let indirect_buffer = generator.indirect_buffer();

                // Bind the command to update the storage buffers
                generator.generate_vertices(
                    renderer,
                    vertex_buffer.clone(),
                    indirect_buffer.clone(),
                    &metaballs,
                );

                // Rendering
                renderer.start_render_pass(&mut camera);

                let info: DrawInfo<BasicVertex> = DrawInfo::IndirectBlank{ indirect_commands: indirect_buffer.clone() };
                renderer.draw_lit(
                    info,
                    generator.graphics_pipeline().clone(), 
                    generator.graphics_descriptors(vertex_buffer, renderer, &params)
                ).unwrap();

                renderer.draw_ambient_light(&mut ambient_light);
                for point_light in point_lights.iter_mut() {
                    renderer.draw_point_light(point_light);
                }

                renderer.end_render_pass();
            }
            _ => (),
        }
    });
}
