use ash::version::DeviceV1_0;
use log::info;
use winit::{
  dpi::LogicalSize,
  event::{Event, VirtualKeyCode, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

use crate::{render_graph::RenderGraph, scene::load_scene, vk_ctx::vk_ctx_initialize};

mod render_graph;
mod scene;
mod vk_ctx;
mod vk_utils;

// glslangValidator.exe -V src/shaders/triangle.frag.glsl src/shaders/triangle.vert.glsl
// spirv-dis.exe vert.spv

fn main() {
  simple_logger::SimpleLogger::new().init().unwrap();
  log::set_max_level(log::LevelFilter::Trace);
  info!("-- Start --");

  // init window
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Rust TressFX")
    .with_resizable(false)
    .with_inner_size(LogicalSize::new(800f64, 600f64))
    .build(&event_loop)
    .unwrap();

  // init renderer
  let mut vk_app = vk_ctx_initialize(&window);
  info!("Render init went OK!");
  let render_graph = RenderGraph::new(&vk_app);

  // scene
  let scene = load_scene(&vk_app);

  // last pre-run ops
  info!("Starting event loop");
  let mut current_frame_in_flight_idx: usize = 0;

  // start event loop
  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      // on clicked 'x'
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => {
        *control_flow = ControlFlow::Exit;
      }

      // on keyboard
      Event::WindowEvent {
        event: WindowEvent::KeyboardInput { input, .. },
        ..
      } => {
        if input.virtual_keycode == Some(VirtualKeyCode::Escape) {
          *control_flow = ControlFlow::Exit;
        }
      }

      Event::MainEventsCleared => {
        render_graph.execute_render_graph(&vk_app, &scene, current_frame_in_flight_idx);
        current_frame_in_flight_idx = (current_frame_in_flight_idx + 1) % vk_app.frames_in_flight()
      }

      // before destroy
      Event::LoopDestroyed => {
        info!("EventLoop is shutting down");

        let device = &vk_app.device.device;
        unsafe {
          // wait to finish current in-flight
          device.device_wait_idle().unwrap();

          // destroy resources as all frames finished rendering
          scene.destroy(&vk_app.allocator);
          render_graph.destroy(&vk_app.device.device);
          vk_app.destroy();
        }
      }

      // default
      _ => (),
    }
  });
}
