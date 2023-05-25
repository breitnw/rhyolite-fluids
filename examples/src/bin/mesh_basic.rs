
use rhyolite::camera::Camera;
use rhyolite::geometry::mesh::{MeshObject, MeshObjectBuilder};
use rhyolite::lighting::{AmbientLight, PointLight};
use rhyolite::renderer::mesh::DrawInfo;
use rhyolite::transform::Transform;
use rhyolite::Rhyolite;

use winit::event::{Event, WindowEvent};
use nalgebra_glm::{identity, rotate_x, rotate_y, rotate_z, vec3};
use examples::CamRotationMode;
use rhyolite::renderer::Renderer;

fn main() {
    let rhyolite = Rhyolite::mesh();
    let renderer = &rhyolite.renderer;

    let camera_transform = Transform::identity();
    let mut camera = Camera::new(camera_transform, 1.2, 0.02, 100.0);

    // Build the models
    let mut suzanne = MeshObjectBuilder::from_file(
        "examples/models/monkey_smooth.obj",
        &vec3(-1.0, -2.0, -5.0),
        &vec3(1.0, 1.0, 1.0),
        &vec3(1.0, 1.0, 1.0),
        (0.3, 4.0),
    ).build(renderer);

    let mut teapot = MeshObjectBuilder::from_file(
        "examples/models/teapot.obj",
        &vec3(3.0, 2.0, -10.0),
        &vec3(0.5, 0.5, 0.5),
        &vec3(1.0, 1.0, 1.0),
        (1.0, 128.0),
    ).build(renderer);

    let plane = MeshObjectBuilder::from_file(
        "examples/models/plane.obj",
        &vec3(0.0, 5.0, -8.0),
        &vec3(10.0, 10.0, 10.0),
        &vec3(0.5, 0.5, 0.5),
        (0.2, 2.0),
    ).build(renderer);

    let mut torus1 = MeshObjectBuilder::from_file(
        "examples/models/torus.obj",
        &vec3(-5.0, 5.0, -10.0),
        &vec3(3.0, 3.0, 3.0),
        &vec3(0.0, 1.0, 0.0),
        (1.0, 128.0),
    ).build(renderer);

    let mut torus2 = MeshObjectBuilder::from_file(
        "examples/models/torus.obj",
        &vec3(-7.0, 6.0, -11.0),
        &vec3(3.5, 3.5, 3.5),
        &vec3(1.0, 0.0, 0.0),
        (1.0, 128.0),
    ).build(renderer);

    let mut bunny = MeshObjectBuilder::from_file(
        "examples/models/bunny.obj",
        &vec3(5.0, 5.0, -4.0),
        &vec3(14.0, 14.0, 14.0),
        &vec3(1.0, 1.0, 1.0),
        (0.2, 2.0),
    ).build(renderer);

    torus1.transform_mut().set_rotation_mat({
        let mut rotation = identity();
        rotation = rotate_x(&rotation, 0.5);
        rotation = rotate_y(&rotation, 2.1);
        rotation
    });

    torus2.transform_mut().set_rotation_mat({
        let mut rotation = identity();
        rotation = rotate_y(&rotation, 0.6);
        rotation = rotate_z(&rotation, -1.1);
        rotation
    });

    bunny
        .transform_mut()
        .set_rotation_mat(rotate_y(&identity(), -0.6));

    // Lighting
    let mut ambient_light = AmbientLight::new(
        vec3(1.0, 1.0, 1.0),
        0.05,
    );

    let mut directional_lights: Vec<(PointLight, MeshObject<_>)> = vec![
        (vec3(-4.0, 0.0, -2.0), vec3(1.0, 0.0, 0.0), 3.0f32),
        (vec3(0.0, -3.0, -14.0), vec3(0.0, 1.0, 0.0), 8.0f32),
        (vec3(4.0, -2.0, -1.0), vec3(0.0, 0.0, 1.0), 5.0f32),
        (vec3(0.0, -25.0, -5.0), vec3(1.0, 0.9, 0.8), 80.0f32),
    ]
    .iter()
    .map(|f| {
        let obj = MeshObjectBuilder::from_file(
            "examples/models/sphere.obj",
            &f.0,
            &(vec3(0.1, 0.1, 0.1) * f.2.sqrt()),
            &f.1,
            // TODO: shouldn't be necessary to specify specular data here
            (0.2, 2.0),
        ).build(renderer);
        (PointLight::new(f.0, f.1, f.2), obj)
    })
    .collect();

    // Other
    let mut camera_pos = vec3(0.0, 0.0, 0.0);
    let mut camera_euler = vec3(0.0, 0.0, 0.0);

    rhyolite.run(move |event, keyboard, _, _, time, renderer| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                camera.configure(renderer.get_window_size());
            }
            Event::RedrawEventsCleared => {
                suzanne
                    .transform_mut()
                    .set_translation(&vec3(time.current.cos() - 1.0, -0.5, -5.0));
                suzanne.transform_mut().set_rotation_mat({
                    let mut rotation = identity();
                    rotation = rotate_y(&rotation, time.current);
                    rotation = rotate_x(&rotation, time.current / 2.);
                    rotation = rotate_z(&rotation, time.current / 3.);
                    rotation
                });

                teapot.transform_mut().set_rotation_mat({
                    let mut rotation = identity();
                    rotation = rotate_x(&rotation, -time.current);
                    rotation = rotate_y(&rotation, time.current / 5.0);
                    rotation = rotate_z(&rotation, time.current / 2.);
                    rotation
                });

                // Camera movement
                examples::do_camera_movement(
                    CamRotationMode::Mesh,
                    &mut camera,
                    &mut camera_euler,
                    &mut camera_pos,
                    &keyboard,
                    time.delta,
                );

                // Rendering
                renderer.start_render_pass(&mut camera);
                renderer.draw_lit_auto(DrawInfo::Vertex { object: &suzanne });
                renderer.draw_lit_auto(DrawInfo::Vertex { object: &plane });
                renderer.draw_lit_auto(DrawInfo::Vertex { object: &teapot });
                renderer.draw_lit_auto(DrawInfo::Vertex { object: &torus1 });
                renderer.draw_lit_auto(DrawInfo::Vertex { object: &torus2 });
                renderer.draw_lit_auto(DrawInfo::Vertex { object: &bunny });
                renderer.draw_ambient_light(&mut ambient_light);
                for light in directional_lights.iter_mut() {
                    renderer.draw_point_light(&mut light.0);
                }
                // for light in directional_lights.iter_mut() {
                //     // TODO: should ideally use an instancing method instead of this, buffer recreated multiple times per frame
                //     renderer.draw_unlit(DrawType::Vertex(&light.1), None);
                // }
                renderer.end_render_pass();
            }
            _ => (),
        }
    });
}
