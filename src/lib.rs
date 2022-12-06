#![allow(dead_code)]



use std::{time::Instant};

pub use renderer::Renderer;
use winit::{event_loop::{EventLoop, ControlFlow, EventLoopWindowTarget}, event::{Event, WindowEvent}};

pub mod shaders;
pub mod transform;
pub mod geometry;
pub mod lighting;
pub mod camera;

mod renderer;


// TODO: implement frames in flight if not implemented in the tutorial
// TODO: replace PersistentDescriptorSet instances with a type expected to be shorter-lived

pub struct Rhyolite {
    pub renderer: Renderer,
    event_loop: Option<EventLoop<()>>,
}

impl Rhyolite {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();
        let renderer = Renderer::new(&event_loop);
        Rhyolite { renderer, event_loop: Some(event_loop) }
    }

    pub fn run<F>(mut self, mut handler: F)
    where F: 'static + FnMut(Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow, &TimeState, &mut Renderer) {
        let mut time_state = TimeState::new();

        self.event_loop.take().unwrap().run(move |event, target, control_flow| {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                    self.renderer.recreate_swapchain();
                },
                Event::RedrawEventsCleared => time_state.update(),
                _ => (),
            }
            handler(event, target, control_flow, &time_state, &mut self.renderer);
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
            delta: 0.001,
        }
    }
    pub(crate) fn update(&mut self) {
        let new_time = self.start_instant.elapsed().as_secs_f32();
        // Clamp delta time to 0.001 to prevent division by 0 errors at the beginning of the program
        self.delta = (new_time - self.current).max(0.001);
        self.current = new_time
    }
}