use log::{info, trace, warn};
use winit::{
  dpi::LogicalSize,
  event::{DeviceEvent, Event, MouseButton},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

use crate::{
  app_input::AppInput, app_timer::AppTimer, app_ui::AppUI, config::Config,
  gpu_profiler::GpuProfiler, render_graph::RenderGraph, scene::load_scene,
  vk_ctx::vk_ctx_initialize,
};

mod app_input;
mod app_timer;
mod app_ui;
mod config;
mod gpu_profiler;
mod render_graph;
mod scene;
mod utils;
mod vk_ctx;
mod vk_utils;

fn main() {
  simple_logger::SimpleLogger::new()
    .with_module_level("imgui_rs_vulkan_renderer", log::LevelFilter::Debug)
    .init()
    .unwrap();
  log::set_max_level(log::LevelFilter::Trace);
  info!("-- Start --");

  // config
  let mut config = Config::new();
  let mut timer = AppTimer::new();

  // init window
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Rust TressFX")
    .with_transparent(Config::TEST_ALPHA_COMPOSITE) // error - see capabilities
    .with_decorations(!Config::TEST_ALPHA_COMPOSITE) // no decorations for alpha compose
    // .with_position(winit::dpi::PhysicalPosition { x: 2500, y: 100 })
    .with_resizable(false)
    .with_inner_size(LogicalSize::new(config.window_width, config.window_height))
    .build(&event_loop)
    .unwrap();
  let mut app_input = AppInput::new();
  info!("Window init: OK!");

  // init vulkan: create device, init structures etc.
  let mut vk_app = vk_ctx_initialize(&window, !config.is_release(), config.vsync());
  info!("Vulkan init: OK!");
  let mut profiler = GpuProfiler::new(&vk_app);

  // scene
  let mut scene = load_scene(&vk_app, &config);
  info!("Scene init: OK!");

  // render graph
  let mut render_graph = RenderGraph::new(&vk_app, &config);
  info!("Render Graph init: OK!");

  // ui
  let mut app_ui = AppUI::new(&window, &vk_app, render_graph.get_ui_draw_render_pass());
  info!("ui init: OK!");

  // last pre-run ops
  info!("Starting event loop");
  let mut current_frame_in_flight_idx: usize = 0;

  // start event loop
  event_loop.run(move |event, _, control_flow| {
    // https://docs.rs/winit/0.25.0/winit/#event-handling
    // *control_flow = ControlFlow::Wait;
    *control_flow = ControlFlow::Poll;

    app_ui.handle_event(&window, &event);
    let imgui_intercepted = app_ui.intercepted_event();
    app_input.handle_event(&event, imgui_intercepted);

    match event {
      // cursor moved - part of rotate. Has to be here, as update inside render loop
      // can get janky and error prone (e.g. some events are skipped 'between' frames?).
      Event::DeviceEvent {
        event: DeviceEvent::MouseMotion { delta, .. },
        ..
      } if !imgui_intercepted => {
        if app_input.is_mouse_button_pressed(MouseButton::Left) {
          // info!("Mouse delta {:?}", delta);
          scene
            .camera
            .rotate_yaw_pitch(delta.0 as f32, delta.1 as f32);
        }
      }
      // redraw
      Event::MainEventsCleared => {
        // https://github.com/EmbarkStudios/kajiya/blob/main/crates/lib/kajiya-simple/src/main_loop.rs#L308
        timer.mark_start_frame();
        profiler.set_enabled(config.profile_next_frame);
        config.profile_next_frame = false;

        // apply events since last frame. "Game logic" in render loop.
        app_input.update_camera_position(&mut scene.camera);
        if config.reset_tfx_simulation_next_frame {
          for tfx_entity in &scene.tressfx_objects {
            tfx_entity.reset_simulation(&vk_app);
          }
        }
        config.reset_tfx_simulation_next_frame = false;

        render_graph.execute_render_graph(
          &window,
          &vk_app,
          current_frame_in_flight_idx,
          &mut config,
          &mut scene,
          &mut app_ui,
          &timer,
          &mut profiler,
        );
        current_frame_in_flight_idx = (current_frame_in_flight_idx + 1) % vk_app.frames_in_flight();

        // clear input events after processed
        app_input.reset_transient_state();

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
          profiler.destroy(vk_app.vk_device());
          trace!("Destroying vulkan objects");
          vk_app.destroy();
        }
      }

      // default
      _ => (),
    }

    if app_input.close_requested {
      *control_flow = ControlFlow::Exit;
    }
  });
}
