use nalgebra_glm::{identity, normalize, rotate_x, rotate_y, rotate_z, vec3, Vec3};
use rhyolite::camera::Camera;
use std::f32::consts;
use winit::event::VirtualKeyCode;

use rhyolite::input::Keyboard;

pub enum KeyBinding {
    WASD,
    ARROWS,
}

// TODO: get_axis() method, move to Keyboard struct

pub fn get_axes(keyboard: &Keyboard, key_binding: KeyBinding) -> Vec3 {
    match key_binding {
        KeyBinding::WASD => vec3(
            (keyboard.key_down(VirtualKeyCode::D) as i32
                - keyboard.key_down(VirtualKeyCode::A) as i32) as f32,
            (keyboard.key_down(VirtualKeyCode::LShift) as i32
                - keyboard.key_down(VirtualKeyCode::Space) as i32) as f32,
            (keyboard.key_down(VirtualKeyCode::S) as i32
                - keyboard.key_down(VirtualKeyCode::W) as i32) as f32,
        ),
        KeyBinding::ARROWS => vec3(
            (keyboard.key_down(VirtualKeyCode::Down) as i32
                - keyboard.key_down(VirtualKeyCode::Up) as i32) as f32,
            (keyboard.key_down(VirtualKeyCode::Left) as i32
                - keyboard.key_down(VirtualKeyCode::Right) as i32) as f32,
            0.0,
        ),
    }
}

pub enum CamRotationMode {
    Mesh,
    Marched,
}

pub fn do_camera_movement(
    rotation_mode: CamRotationMode,
    camera: &mut Camera,
    camera_euler: &mut Vec3,
    camera_pos: &mut Vec3,
    keyboard: &Keyboard,
    delta_time: f32,
) {
    const CAM_MOVE_SPEED: f32 = 4.0;
    const CAM_ROT_SPEED: f32 = 0.6;

    let camera_move = get_axes(keyboard, KeyBinding::WASD);
    let camera_rotate = get_axes(keyboard, KeyBinding::ARROWS);

    let (do_move, do_rotate) = (
        camera_move.magnitude() != 0.0,
        camera_rotate.magnitude() != 0.0,
    );
    if do_move || do_rotate {
        let transform = camera.transform_mut();
        if do_rotate {
            *camera_euler += normalize(&camera_rotate) * CAM_ROT_SPEED * delta_time;
            camera_euler.x = camera_euler.x.clamp(-consts::PI / 2.0, consts::PI / 2.0);

            // TODO: THIS IS FUCKED (it works tho)
            transform.set_rotation_mat(
                match rotation_mode {
                    CamRotationMode::Mesh => rotate_z(&rotate_x(&rotate_y(&identity(), camera_euler.y), camera_euler.x), camera_euler.z),
                    CamRotationMode::Marched => rotate_y(&rotate_x(&rotate_z(&identity(), camera_euler.z), camera_euler.x), camera_euler.y),
                },
            );
        }
        if do_move {
            *camera_pos += nalgebra_glm::rotate_y_vec3(&normalize(&camera_move), camera_euler.y)
                * CAM_MOVE_SPEED
                * delta_time;
            transform.set_translation(&camera_pos);
        }
    }
}
