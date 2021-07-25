use log::{error, info};
use winit::{
  dpi::LogicalSize,
  event::{Event, VirtualKeyCode, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

mod renderer;
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
    // TODO do not show window yet
    .unwrap();

  // init renderer
  unsafe {
    let result = renderer::main::main(&window);
    match result {
      Err(err) => {
        error!("Render init error - something went wrong");
        eprintln!("error: {:?}", err);
        std::process::exit(1);
      }
      _ => {
        info!("Render init went OK!");
      }
    }
  }

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
        // TODO draw here
        // https://github.com/expenses/vulkan-base/blob/main/src/main.rs#L379
      }

      // before destroy
      Event::LoopDestroyed => {
        info!("EventLoop is shutting down");
      }

      // default
      _ => (),
    }
  });
}
