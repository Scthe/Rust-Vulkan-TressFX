use ash;
use ash::vk;
use imgui::{Condition, Context, TreeNodeFlags, Ui};
use imgui_rs_vulkan_renderer::{Options, Renderer};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use winit::event::Event;

use crate::{config::Config, vk_ctx::VkCtx};

pub const WIDGET_HALF: f32 = 150.0;

/// Controls examples: https://magnum.graphics/showcase/imgui/
///
/// Why mouse has to be moved after click etc.:
/// https://github.com/rib/gputop/issues/172
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
  pub fn render_ui(
    &mut self,
    window: &winit::window::Window,
    command_buffer: vk::CommandBuffer,
    config: &mut Config,
  ) {
    self
      .platform
      .prepare_frame(self.imgui.io_mut(), &window)
      .expect("Failed to prepare frame");
    {
      let ui = self.imgui.frame();

      // UI START
      ui.window("Settings")
        .position([0.0, 0.0], Condition::Always)
        .movable(false)
        .size([300.0, 500.0], Condition::Always)
        .resizable(false)
        .build(|| {
          // let _ = ui.push_item_width(150.0); // TODO does not work? How to set it as default?

          AppUI::draw_general_ui(ui, config);
          ui.spacing();
          AppUI::draw_fxaa_ui(ui, config);
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

  fn draw_general_ui(ui: &Ui, config: &mut Config) {
    ui.combo(
      "Display mode",
      &mut config.display_mode,
      &[Config::DISPLAY_MODE_FINAL, Config::DISPLAY_MODE_NORMALS],
      |idx| {
        if *idx == Config::DISPLAY_MODE_NORMALS {
          std::borrow::Cow::Borrowed("Normals")
        } else {
          std::borrow::Cow::Borrowed("Final")
        }
      },
    );
  }

  fn draw_fxaa_ui(ui: &Ui, config: &mut Config) {
    let flags: TreeNodeFlags =
      TreeNodeFlags::FRAMED | TreeNodeFlags::FRAME_PADDING | TreeNodeFlags::SPAN_FULL_WIDTH;

    if ui.collapsing_header("Fxaa", flags) {
      ui.checkbox("Use FXAA", &mut config.postfx.use_fxaa);
      next_widget_small(ui);
      ui.slider("Subpixel aa", 0.0, 1.0, &mut config.postfx.subpixel);
      next_widget_small(ui);
      ui.slider(
        "Contrast Treshold",
        0.063,
        0.333,
        &mut config.postfx.edge_threshold,
      );
      next_widget_small(ui);
      ui.slider(
        "Edge Treshold",
        0.0,
        0.0833,
        &mut config.postfx.edge_threshold_min,
      );
    }
  }
}

fn next_widget_small(ui: &Ui) {
  let _ = ui.set_next_item_width(WIDGET_HALF);
}
