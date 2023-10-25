use glam::{vec3, Vec3};
use log::{info, trace, warn};
use winit::{
  dpi::LogicalSize,
  event::{
    DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
  },
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

use crate::{
  app_ui::AppUI, config::Config, render_graph::RenderGraph, scene::load_scene,
  vk_ctx::vk_ctx_initialize,
};

mod app_ui;
mod config;
mod render_graph;
mod scene;
mod utils;
mod vk_ctx;
mod vk_utils;

// glslangValidator.exe -V src/shaders/triangle.frag.glsl src/shaders/triangle.vert.glsl
// spirv-dis.exe vert.spv

fn main() {
  simple_logger::SimpleLogger::new()
    .with_module_level("imgui_rs_vulkan_renderer", log::LevelFilter::Debug)
    .init()
    .unwrap();
  log::set_max_level(log::LevelFilter::Trace);
  info!("-- Start --");

  // config
  let mut config = Config::new();

  // init window
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Rust TressFX")
    .with_resizable(false)
    .with_inner_size(LogicalSize::new(config.window_width, config.window_height))
    .build(&event_loop)
    .unwrap();
  info!("Window init: OK!");

  // init vulkan: create device, init structures etc.
  let mut vk_app = vk_ctx_initialize(&window);
  info!("Vulkan init: OK!");

  // scene
  let mut scene = load_scene(&vk_app, &config);
  info!("Scene init: OK!");

  // render graph
  let mut render_graph = RenderGraph::new(&vk_app, &config);
  info!("Render Graph init: OK!");

  // ui
  let mut app_ui = AppUI::new(&window, &vk_app, render_graph.get_last_render_pass());
  info!("ui init: OK!");

  // last pre-run ops
  info!("Starting event loop");
  let mut current_frame_in_flight_idx: usize = 0;
  let mut is_left_mouse_button_pressed = false;

  // start event loop
  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    app_ui.handle_event(&window, &event);

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
        if app_ui.intercepted_event() {
          return;
        }
        if input.virtual_keycode == Some(VirtualKeyCode::Escape) {
          *control_flow = ControlFlow::Exit;
        }
        if input.virtual_keycode == Some(VirtualKeyCode::F) {
          // debug
          let (side, up, forward) = scene.camera.get_rotation_axes();
          let pos = scene.camera.position();
          info!(
            "Camera(pos={} side={}, up={}, forward={})",
            pos, side, up, forward
          )
        }

        let camera_move = parse_camera_move_key_code(input.virtual_keycode);
        scene.camera.move_(camera_move);
      }

      // mouse wheel
      Event::WindowEvent {
        // TODO use DeviceEvent not WindowEvent?
        event: WindowEvent::MouseWheel { delta, .. },
        ..
      } => {
        if app_ui.intercepted_event() {
          return;
        }
        if let MouseScrollDelta::LineDelta(_, delta_y) = delta {
          scene.camera.move_forward(-delta_y);
        }
      }

      // cursor moved - part of rotate
      Event::DeviceEvent {
        event: DeviceEvent::MouseMotion { delta, .. },
        ..
      } => {
        if app_ui.intercepted_event() {
          return;
        }
        if is_left_mouse_button_pressed {
          // info!("Mouse delta {:?}", delta);
          scene
            .camera
            .rotate_yaw_pitch(delta.0 as f32, delta.1 as f32);
        }
      }

      // mouse buttons
      Event::WindowEvent {
        event: WindowEvent::MouseInput { button, state, .. },
        ..
      } => {
        if app_ui.intercepted_event() {
          return;
        }
        // info!("button={:?}, state={:?}", button, state);
        if button == MouseButton::Left {
          is_left_mouse_button_pressed = state == ElementState::Pressed;
        }
      }

      // window focus
      Event::WindowEvent {
        event: WindowEvent::Focused(is_focused),
        ..
      } => {
        info!("Window focus change. Are we in focus: {:?}", is_focused);
        is_left_mouse_button_pressed = false;
      }

      // window focus
      Event::WindowEvent {
        event: WindowEvent::CursorLeft { .. },
        ..
      } => {
        info!("Cursor left the window");
        is_left_mouse_button_pressed = false;
      }

      // redraw
      Event::MainEventsCleared => {
        render_graph.execute_render_graph(
          &vk_app,
          &mut config,
          &scene,
          current_frame_in_flight_idx,
          &mut app_ui,
          &window,
        );
        current_frame_in_flight_idx = (current_frame_in_flight_idx + 1) % vk_app.frames_in_flight();

        if config.only_first_frame {
          warn!("Closing app as `Config.only_first_frame` is set to true!");
          *control_flow = ControlFlow::Exit;
        }
      }

      // before destroy
      Event::LoopDestroyed => {
        info!("EventLoop is shutting down");

        let device = vk_app.vk_device();
        unsafe {
          // wait to finish current in-flight
          device.device_wait_idle().unwrap();

          // destroy resources as all frames finished rendering
          trace!("Destroying scene objects");
          scene.destroy(vk_app.vk_device(), &vk_app.allocator);
          trace!("Destroying render graph objects");
          render_graph.destroy(&vk_app);
          trace!("Destroying vulkan objects");
          vk_app.destroy();
        }
      }

      // default
      _ => (),
    }
  });
}

fn parse_camera_move_key_code(keycode_opt: Option<VirtualKeyCode>) -> Vec3 {
  match keycode_opt {
    Some(keycode) if keycode == VirtualKeyCode::W => vec3(0f32, 0f32, -1f32),
    Some(keycode) if keycode == VirtualKeyCode::S => vec3(0f32, 0f32, 1f32),
    Some(keycode) if keycode == VirtualKeyCode::A => vec3(-1f32, 0f32, 0f32),
    Some(keycode) if keycode == VirtualKeyCode::D => vec3(1f32, 0f32, 0f32),
    Some(keycode) if keycode == VirtualKeyCode::Space => vec3(0f32, 1f32, 0f32),
    Some(keycode) if keycode == VirtualKeyCode::Z => vec3(0f32, -1f32, 0f32),
    _ => Vec3::ZERO,
  }
}
