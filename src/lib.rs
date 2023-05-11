#![allow(dead_code)]

extern crate core;

use core::time;
use std::time::Instant;

use crate::input::Keyboard;
use renderer::{/*marched::MarchedRenderer,*/ mesh::MeshRenderer, Renderer};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

pub mod camera;
pub mod geometry;
pub mod input;
pub mod lighting;
pub mod renderer;
pub mod shaders;
pub mod transform;

// TODO: implement frames in flight if not implemented in the tutorial
// TODO: replace PersistentDescriptorSet instances with a type expected to be shorter-lived

/// The base struct of all Rhyolite operations.
pub struct Rhyolite<T: Renderer> {
    pub renderer: T,
    event_loop: Option<EventLoop<()>>,
}

impl Rhyolite<MeshRenderer> {
    /// Creates a new Rhyolite mesh renderer with a specified Winit event loop.
    pub fn mesh() -> Rhyolite<MeshRenderer> {
        let event_loop = EventLoop::new();
        let renderer = MeshRenderer::new(&event_loop);
        Rhyolite {
            renderer,
            event_loop: Some(event_loop),
        }
    }
}

// impl Rhyolite<MarchedRenderer> {
//     /// Creates a new Rhyolite ray marched renderer with a specified Winit event loop.
//     pub fn ray_marched() -> Rhyolite<MarchedRenderer> {
//         let event_loop = EventLoop::new();
//         let renderer = MarchedRenderer::new(&event_loop);
//         Rhyolite {
//             renderer,
//             event_loop: Some(event_loop),
//         }
//     }
// }

impl<T: Renderer + 'static> Rhyolite<T> {
    /// Runs a FnMut closure with the Rhyolite instance. Events for program exiting, swapchain recreation on resize, and TimeState calculation are executed before
    /// the closure is called.
    pub fn run<F>(mut self, mut handler: F)
    where
        F: 'static
            + FnMut(
                Event<'_, ()>,
                &Keyboard,
                &EventLoopWindowTarget<()>,
                &mut ControlFlow,
                &TimeState,
                &mut T,
            ),
    {
        let mut time_state = TimeState::new();
        let mut keyboard = Keyboard::new();

        let mut occluded = false;

        self.event_loop
            .take()
            .unwrap()
            .run(move |event, target, control_flow| {
                match &event {
                    Event::WindowEvent { event, ..} => match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            self.renderer.recreate_all_size_dependent();
                        }
                        WindowEvent::Resized(_) => {
                            self.renderer.recreate_all_size_dependent();
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            keyboard.update_with_input(input);
                        }
                        WindowEvent::Occluded(val) => {
                            occluded = *val;
                        }
                        _ => ()
                    }
                    Event::RedrawEventsCleared => time_state.update(),
                    _ => (),
                }

                if occluded {
                    std::thread::sleep(time::Duration::from_millis(1000 / 60));
                    return;
                }

                handler(
                    event,
                    &keyboard,
                    target,
                    control_flow,
                    &time_state,
                    &mut self.renderer,
                );
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

/// A struct representing various time-related values, automatically calculated by Rhyolite before each frame.
/// * `current`: The amount of time elapsed since the start of the program, in seconds.
/// * `delta`: The amount of time elapsed since the last frame, in seconds.
pub struct TimeState {
    start_instant: Instant,
    pub current: f32,
    pub delta: f32,
}
impl TimeState {
    pub(crate) fn new() -> Self {
        TimeState {
            start_instant: Instant::now(),
            current: 0.0,
            delta: 0.0,
        }
    }
    pub(crate) fn update(&mut self) {
        let new_time = self.start_instant.elapsed().as_secs_f32();
        self.delta = new_time - self.current;
        self.current = new_time
    }
}
