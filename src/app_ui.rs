use ash;
use ash::vk;
use imgui::{Condition, Context};
use imgui_rs_vulkan_renderer::{Options, Renderer};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use winit::event::Event;

use crate::vk_ctx::VkCtx;

pub struct AppUI {
  imgui: imgui::Context,
  renderer: Renderer,
  platform: WinitPlatform,
}

impl AppUI {
  pub fn new(window: &winit::window::Window, vk_app: &VkCtx, render_pass: vk::RenderPass) -> Self {
    let mut imgui = Context::create();
    let mut platform = WinitPlatform::init(&mut imgui);
    platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);

    // TODO could be better if we used same allocator as rest of app, but..
    let renderer = Renderer::with_default_allocator(
      &vk_app.instance,
      vk_app.device.phys_device,
      vk_app.device.device.clone(),
      vk_app.device.queue,
      vk_app.command_buffers.pool,
      render_pass,
      &mut imgui,
      Some(Options {
        in_flight_frames: vk_app.frames_in_flight(),
        ..Default::default()
      }),
    )
    .expect("Failed to initialize GUI");

    Self {
      imgui,
      renderer,
      platform,
    }
  }

  pub fn handle_event(&mut self, window: &winit::window::Window, event: &Event<()>) {
    self
      .platform
      .handle_event(self.imgui.io_mut(), window, &event);
  }

  /// https://github.com/Scthe/WebFX/blob/master/src/UISystem.ts
  pub fn render_ui(&mut self, window: &winit::window::Window, command_buffer: vk::CommandBuffer) {
    self
      .platform
      .prepare_frame(self.imgui.io_mut(), &window)
      .expect("Failed to prepare frame");
    {
      let ui = self.imgui.frame();

      // UI START
      ui.window("Settings")
        .position([0.0, 0.0], Condition::FirstUseEver)
        .movable(false)
        .size([300.0, 110.0], Condition::FirstUseEver)
        .resizable(false)
        .build(|| {
          ui.text_wrapped("Hello world!");
          ui.text_wrapped("こんにちは世界！");
          ui.separator();
          ui.button("This...is...imgui-rs!");
        });

      // UI END
      self.platform.prepare_render(&ui, &window);
    }

    let draw_data = self.imgui.render();
    self
      .renderer
      .cmd_draw(command_buffer, draw_data)
      .expect("Failed to render ui");
  }
}
