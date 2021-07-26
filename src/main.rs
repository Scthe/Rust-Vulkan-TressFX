use ash::version::DeviceV1_0;
use log::info;
use winit::{
  dpi::LogicalSize,
  event::{Event, VirtualKeyCode, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};

mod renderer;
mod vk_app;
mod vk_init;
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
  let vk_app = vk_init::vk_init(&window);
  info!("Render init went OK!");
  unsafe {
    info!("Starting render loop");
    renderer::main::render_loop(&vk_app);

    info!("Sync: device_wait_idle()");
    vk_app
      .device
      .device
      .device_wait_idle()
      .expect("Failed device_wait_idle()");
  };

  info!("Starting event loop");

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
        unsafe { vk_app.destroy() };
      }

      // default
      _ => (),
    }
  });
}
