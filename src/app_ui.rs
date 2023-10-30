use ash;
use ash::vk;
use imgui::{ColorEditFlags, Condition, Context, TreeNodeFlags, Ui};
use imgui_rs_vulkan_renderer::{Options, Renderer};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use lazy_static::lazy_static;
use std::borrow::Cow;
use winit::event::Event;

use crate::{
  config::{
    ColorGradingPerRangeSettings, ColorGradingProp, Config, DisplayMode, PostFxCfg, SSAOConfig,
    ShadowTechnique, ShadowsConfig, TonemappingMode,
  },
  vk_ctx::VkCtx,
};

const WIDGET_HALF: f32 = 150.0;

lazy_static! {
  static ref HEADER_FLAGS: TreeNodeFlags =
    TreeNodeFlags::FRAMED | TreeNodeFlags::FRAME_PADDING | TreeNodeFlags::SPAN_FULL_WIDTH;
  static ref COLOR_FLAGS: ColorEditFlags = ColorEditFlags::NO_INPUTS | ColorEditFlags::INPUT_HSV;
}

/// Controls examples:
/// - https://magnum.graphics/showcase/imgui/
///
/// In rust:
/// - https://github.com/adrien-ben/imgui-rs-vulkan-renderer/blob/master/examples/color_button.rs
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

  pub fn intercepted_event(&self) -> bool {
    let io = self.imgui.io();
    io.want_capture_mouse || io.want_capture_keyboard || io.want_text_input
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
          Self::draw_general_ui(ui, config);
          ui.spacing();

          Self::draw_shadows(ui, &mut config.shadows);
          Self::draw_ssao(ui, &mut config.ssao);
          let postfx = &mut config.postfx;
          Self::draw_post_fx(ui, postfx);
          {
            let cg = &mut postfx.color_grading;
            let sm = Some((&mut cg.shadows_max, "shadowsMax"));
            let hl = Some((&mut cg.highlights_min, "highlightsMin"));
            Self::draw_color_grading(ui, "general", &mut cg.global, None);
            Self::draw_color_grading(ui, "shadows", &mut cg.shadows, sm);
            Self::draw_color_grading(ui, "midtones", &mut cg.midtones, None);
            Self::draw_color_grading(ui, "highlights", &mut cg.highlights, hl);
          }
          Self::draw_fxaa_ui(ui, config);
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
    let push_token = ui.push_id("GeneralUI");

    ui.combo(
      "Display mode",
      &mut config.display_mode,
      &[
        DisplayMode::Final,
        DisplayMode::Normals,
        DisplayMode::Luma,
        DisplayMode::SSAO,
        DisplayMode::LinearDepth, // TODO ???
        DisplayMode::ShadowMap,
      ],
      |idx| match *idx {
        DisplayMode::Normals => Cow::Borrowed("Normals"),
        DisplayMode::Luma => Cow::Borrowed("Luma"),
        DisplayMode::SSAO => Cow::Borrowed("SSAO"),
        DisplayMode::LinearDepth => Cow::Borrowed("Linear depth"),
        DisplayMode::ShadowMap => Cow::Borrowed("Shadows"),
        _ => Cow::Borrowed("Final"),
      },
    );

    if config.display_mode == DisplayMode::LinearDepth as _ {
      ui.slider(
        "Near",
        -config.camera.z_near,
        -config.camera.z_far,
        &mut config.linear_depth_preview_range.x,
      );
      ui.slider(
        "Far",
        -config.camera.z_near,
        -config.camera.z_far,
        &mut config.linear_depth_preview_range.y,
      );
    }

    ui.checkbox("Show positions", &mut config.show_debug_positions);
    add_tooltip_to_previous_widget(ui, "Show positions of lights and shadow source");

    push_token.end();
  }

  fn draw_shadows(ui: &Ui, shadows: &mut ShadowsConfig) {
    let push_token = ui.push_id("shadows");

    if ui.collapsing_header("Shadows", *HEADER_FLAGS) {
      // dir.add(this.cfg.shadows, 'showDebugView').name('Show dbg');
      ui.combo(
        "Technique",
        &mut shadows.shadow_technique,
        &[
          ShadowTechnique::BinaryDebug,
          ShadowTechnique::PFC,
          ShadowTechnique::PCSS,
        ],
        |idx| match *idx {
          ShadowTechnique::BinaryDebug => Cow::Borrowed("Binary debug"),
          ShadowTechnique::PFC => Cow::Borrowed("PFC"),
          _ => Cow::Borrowed("PCSS"),
        },
      );
      add_tooltip_to_previous_widget(
        ui,
        "Use Percentage-Closer Soft Shadows or Percentage-closer Filtering or simplest possible binary debug check",
      );
      if shadows.shadow_technique == (ShadowTechnique::PFC as _) {
        ui.slider("Blur radius", 0, 4, &mut shadows.blur_radius);
      }
      ui.slider("Strength", 0.0, 1.0, &mut shadows.strength);
      ui.slider("Bias", 0.001, 0.01, &mut shadows.bias);
      // dir.add(this.cfg.shadows, 'blurRadiusTfx', [0, 1, 2, 3, 4]).name('HAIR Blur radius');
      // dir.add(this.cfg.shadows, 'biasHairTfx', 0.001, 0.01).name('HAIR Bias');
      // dir.add(this.cfg.shadows, 'hairTfxRadiusMultipler', 0.5, 3.0).name('HAIR Radius mul');
      ui.slider(
        "Position phi",
        -179.0,
        179.0,
        &mut shadows.shadow_source.pos_phi,
      );
      ui.slider(
        "Position th",
        15.0,
        165.0,
        &mut shadows.shadow_source.pos_theta,
      );
      // dir.add(this.cfg.shadows.directionalLight, 'posRadius', 1, 10).step(0.1).name('Position r');
    }

    push_token.end();
  }

  fn draw_ssao(ui: &Ui, ssao: &mut SSAOConfig) {
    let push_token = ui.push_id("ssao");

    if ui.collapsing_header("SSAO", *HEADER_FLAGS) {
      next_widget_small(ui);
      ui.slider(
        "Kernel size",
        1,
        SSAOConfig::MAX_KERNEL_VALUES,
        &mut ssao.kernel_size,
      );
      next_widget_small(ui);
      ui.slider("Radius", 0.1, 3.0, &mut ssao.radius);
      next_widget_small(ui);
      ui.slider("Bias", 0.0, 0.1, &mut ssao.bias);
      next_widget_small(ui);
      ui.slider("Blur radius", 0, 9, &mut ssao.blur_radius);
      next_widget_small(ui);
      ui.slider("Blur gauss sigma", 1.0, 6.0, &mut ssao.blur_gauss_sigma); // delta 0.1
      next_widget_small(ui);
      ui.slider(
        "Blur depth diff",
        0.01,
        0.4,
        &mut ssao.blur_max_depth_distance,
      );
      next_widget_small(ui);
      ui.slider("AO strength", 0.0, 1.0, &mut ssao.ao_strength); // delta 0.01
      next_widget_small(ui);
      ui.slider("AO exp", 0.0, 5.0, &mut ssao.ao_exp); // delta 0.1
    }

    push_token.end();
  }

  fn draw_color_grading(
    ui: &Ui,
    label: &str,
    postfx: &mut ColorGradingPerRangeSettings,
    slider: Option<(&mut f32, &str)>,
  ) {
    let title = format!("Color grading - {}", label);
    let push_token = ui.push_id(title.clone());

    if ui.collapsing_header(title, *HEADER_FLAGS) {
      if let Some(mut s) = slider {
        next_widget_small(ui);
        ui.slider(s.1, 0.0, 1.0, &mut s.0);
      }
      Self::draw_color_grading_prop(ui, "Saturation", 0.0, 2.0, &mut postfx.saturation);
      Self::draw_color_grading_prop(ui, "Contrast", 0.0, 2.0, &mut postfx.contrast);
      Self::draw_color_grading_prop(ui, "Gamma", 0.0, 2.0, &mut postfx.gamma);
      Self::draw_color_grading_prop(ui, "Gain", 0.0, 2.0, &mut postfx.gain);
      Self::draw_color_grading_prop(ui, "Offset", -1.0, 1.0, &mut postfx.offset);
    }

    push_token.end();
  }

  fn draw_color_grading_prop(
    ui: &Ui,
    label: &str,
    min: f32,
    max: f32,
    prop: &mut ColorGradingProp,
  ) {
    ui.color_edit3_config(format!("##{}-color", label), &mut prop.color)
      .flags(*COLOR_FLAGS)
      .build();
    ui.same_line();
    next_widget_small(ui);
    ui.slider(label, min, max, &mut prop.value);
  }

  fn draw_post_fx(ui: &Ui, postfx: &mut PostFxCfg) {
    let push_token = ui.push_id("PostFX");

    if ui.collapsing_header("PostFX", *HEADER_FLAGS) {
      ui.slider("Dither", 0.0, 2.0, &mut postfx.dither_strength);

      ui.combo(
        "Tonemapping",
        &mut postfx.tonemapping_op,
        &[
          TonemappingMode::Linear,
          TonemappingMode::Reinhard,
          TonemappingMode::Uncharted2,
          TonemappingMode::Photographic,
          TonemappingMode::AcesUe4,
        ],
        |idx| match *idx {
          TonemappingMode::Linear => Cow::Borrowed("Linear"),
          TonemappingMode::Reinhard => Cow::Borrowed("Reinhard"),
          TonemappingMode::Uncharted2 => Cow::Borrowed("Uncharted2"),
          TonemappingMode::Photographic => Cow::Borrowed("Photographic"),
          _ => Cow::Borrowed("ACES_UE4"),
        },
      );

      if postfx.tonemapping_op == TonemappingMode::AcesUe4 as _ {
        ui.slider("AcesC", 0.5, 1.5, &mut postfx.aces_c);
        ui.slider("AcesS", 0.0, 2.0, &mut postfx.aces_s);
      }
      if postfx.tonemapping_op == TonemappingMode::Linear as _
        || postfx.tonemapping_op == TonemappingMode::Reinhard as _
        || postfx.tonemapping_op == TonemappingMode::Uncharted2 as _
      {
        ui.slider("Exposure", 0.5, 2.0, &mut postfx.exposure);
      }
      if postfx.tonemapping_op == TonemappingMode::Uncharted2 as _
        || postfx.tonemapping_op == TonemappingMode::Photographic as _
      {
        ui.slider("White point", 0.5, 2.0, &mut postfx.white_point);
      }
    }

    push_token.end();
  }

  fn draw_fxaa_ui(ui: &Ui, config: &mut Config) {
    let push_token = ui.push_id("fxaa");

    if ui.collapsing_header("Fxaa", *HEADER_FLAGS) {
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

    push_token.end();
  }
}

fn next_widget_small(ui: &Ui) {
  let _ = ui.set_next_item_width(WIDGET_HALF);
}

fn add_tooltip_to_previous_widget(ui: &Ui, tooltip: &str) {
  if ui.is_item_hovered() {
    ui.tooltip_text(tooltip);
  }
}
