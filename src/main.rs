use log::{error, info, warn};
use winit::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

// mod _shared;
mod renderer;

fn main() {
  println!("Start!");

  simple_logger::init().unwrap(); // .filter_level(log::LevelFilter::Debug).init();

  // pretty_env_logger::init();
  log::set_max_level(log::LevelFilter::Trace);
  // log::set_max_level(log::LevelFilter::Error);
  info!("log::infor");
  warn!("log::warn");
  error!("log::error");

  // init window
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("App 1")
    .with_resizable(false)
    .with_inner_size(winit::dpi::LogicalSize::new(f64::from(800), f64::from(600)))
    .build(&event_loop)
    .unwrap();

  // init renderer
  unsafe {
    renderer::main::main(&window).unwrap();
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
        if input.virtual_keycode == Some(winit::event::VirtualKeyCode::Escape) {
          *control_flow = ControlFlow::Exit;
        }
      }
      // default
      _ => (),
    }
  });
}
