use ash;
use ash::vk;
use glam::{vec4, Vec4};
use imgui::{
  internal::DataTypeKind, ColorEditFlags, Condition, Context, StyleColor, TreeNodeFlags, Ui,
};
use imgui_rs_vulkan_renderer::{Options, Renderer};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use lazy_static::lazy_static;
use std::borrow::Cow;
use winit::event::Event;

use crate::{
  app_timer::AppTimer,
  config::{
    tfx_simulation::TfxSimulation, ColorGradingPerRangeSettings, ColorGradingProp, Config,
    DisplayMode, HairPPLLDisplayMode, HairSolidDisplayMode, HairTechnique, LightAmbient, LightCfg,
    PostFxCfg, SSAOConfig, SSSBlurPassCfg, SSSForwardScatterPassCfg, ShadowTechnique,
    ShadowsConfig, TonemappingMode,
  },
  either,
  gpu_profiler::{GpuProfiler, GpuProfilerReport},
  render_graph::PassExecContext,
  scene::{TfxObject, WorldEntity},
  utils::{first_letters, vec3_to_pretty_str},
  vk_ctx::VkCtx,
};

const WIDGET_HALF: f32 = 150.0;

lazy_static! {
  static ref HEADER_FLAGS: TreeNodeFlags =
    TreeNodeFlags::FRAMED | TreeNodeFlags::FRAME_PADDING | TreeNodeFlags::SPAN_FULL_WIDTH;
}

const SSS_FORWARD_TOOLTIP: &str = "Light that passes through thin parts of model (ears, nose)";

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

    // TODO [LOW] could be better if we used same allocator as rest of app, but..
    let renderer = Renderer::with_default_allocator(
      &vk_app.instance,
      vk_app.device.phys_device,
      vk_app.device.device.clone(),
      vk_app.device.queue,
      vk_app.command_pool,
      render_pass,
      &mut imgui,
      Some(Options {
        in_flight_frames: vk_app.swapchain_images_count(),
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
  pub fn render_ui(&mut self, exec_ctx: &PassExecContext, command_buffer: vk::CommandBuffer) {
    let window = exec_ctx.window;
    let config: &mut Config = &mut exec_ctx.config.borrow_mut();
    let timer = exec_ctx.timer;
    let profiler = &mut exec_ctx.profiler.borrow_mut();
    let scene = &mut exec_ctx.scene.borrow_mut();

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
          Self::draw_general_ui(ui, config, timer);
          Self::draw_hair_settings(ui, config);
          ui.spacing();

          Self::draw_hair_simulation_settings(ui, config);
          scene
            .entities
            .iter_mut()
            .for_each(|entity| Self::draw_entity(ui, entity));
          scene
            .tressfx_objects
            .iter_mut()
            .for_each(|entity| Self::draw_tfx_object(ui, config, entity));
          Self::draw_ambient_light(ui, &mut config.light_ambient);
          Self::draw_light(ui, "Light 0", &mut config.light0);
          Self::draw_light(ui, "Light 1", &mut config.light1);
          Self::draw_light(ui, "Light 2", &mut config.light2);
          Self::draw_shadows(ui, &mut config.shadows);
          Self::draw_sss_forward_pass(ui, &mut config.sss_forward_scatter);
          Self::draw_sss_blur(ui, &mut config.sss_blur);
          Self::draw_ssao(ui, &mut config.ssao);
          let postfx = &mut config.postfx;
          Self::draw_post_fx(ui, postfx);
          {
            let cg = &mut postfx.color_grading;
            let sm = Some((&mut cg.shadows_max, "Shadows max"));
            let hl = Some((&mut cg.highlights_min, "Highlights min"));
            Self::draw_color_grading(ui, "general", &mut cg.global, None);
            Self::draw_color_grading(ui, "shadows", &mut cg.shadows, sm);
            Self::draw_color_grading(ui, "midtones", &mut cg.midtones, None);
            Self::draw_color_grading(ui, "highlights", &mut cg.highlights, hl);
          }
          Self::draw_fxaa_ui(ui, config);
          Self::draw_gpu_profiler(ui, config, profiler);
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

  fn draw_general_ui(ui: &Ui, config: &mut Config, timer: &AppTimer) {
    let push_token = ui.push_id("GeneralUI");

    let dt = timer.delta_time_ms();
    let vsync = either!(config.vsync(), "ON", "OFF");
    ui.text_disabled(format!(
      "Timer: {:.2}ms ({:.0} FPS), vsync: {}",
      dt,
      1000.0 / dt,
      vsync
    ));

    next_widget_small(ui);
    ui.combo(
      "Display mode",
      &mut config.display_mode,
      &[
        DisplayMode::Final,
        DisplayMode::Normals,
        DisplayMode::Luma,
        DisplayMode::SSAO,
        DisplayMode::LinearDepth,
        DisplayMode::ShadowMap,
        DisplayMode::SSSContribution,
        DisplayMode::SSSThickness,
      ],
      |idx| match *idx {
        DisplayMode::Normals => Cow::Borrowed("Normals"),
        DisplayMode::Luma => Cow::Borrowed("Luma"),
        DisplayMode::SSAO => Cow::Borrowed("SSAO"),
        DisplayMode::LinearDepth => Cow::Borrowed("Linear depth"),
        DisplayMode::ShadowMap => Cow::Borrowed("Shadows"),
        DisplayMode::SSSContribution => Cow::Borrowed("SSS contribution"),
        DisplayMode::SSSThickness => Cow::Borrowed("SSS thickness"),
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
    add_tooltip_to_previous_widget(ui, "Show positions of lights, shadow source, wind etc.");

    push_token.end();
  }

  fn draw_gpu_profiler(ui: &Ui, config: &mut Config, profiler: &GpuProfiler) {
    let push_token = ui.push_id("gpu_profiler");

    if ui.collapsing_header("GPU profiler", *HEADER_FLAGS) {
      if ui.button("Profile") {
        config.profile_next_frame = true;
      }

      match profiler.get_last_report() {
        None => ui.text_disabled("No results yet"),
        Some(report) => Self::draw_profiler_report(ui, &report),
      }
    }

    push_token.end();
  }

  fn draw_profiler_report(ui: &Ui, report: &GpuProfilerReport) {
    let name_letters = 17;
    let total_ms = report
      .iter()
      .fold(0f32, |acc, (_, duration_ms)| acc + duration_ms);

    report.iter().for_each(|(name, duration_ms)| {
      let perc = duration_ms / total_ms * 100.0;
      // let name2 = &name[..name_letters];
      let name2 = first_letters(name, name_letters);
      ui.text_disabled(format!(
        "{:<name_letters$}: {:>4.2}ms ({:>5.2}%)",
        name2, duration_ms, perc
      ));
      add_tooltip_to_previous_widget(ui, &name);
    });

    ui.text_disabled(format!("Total: {:.2}ms", total_ms));
  }

  fn draw_hair_settings(ui: &Ui, config: &mut Config) {
    let push_token = ui.push_id("tressfx");

    next_widget_small(ui);
    ui.combo(
      "Hair technique",
      &mut config.hair_technique,
      &[HairTechnique::PPLL, HairTechnique::Solid],
      |idx| match *idx {
        HairTechnique::Solid => Cow::Borrowed("Solid"),
        _ => Cow::Borrowed("PPLL"),
      },
    );
    add_tooltip_to_previous_widget(ui,
      "PPLL - Order-Independent Transparency using Per-Pixel Linked List (TressFX)\nSolid - closest fragment wins, no alpha"
    );

    if config.display_mode == (DisplayMode::Final as _) {
      if config.hair_technique == HairTechnique::PPLL as _ {
        Self::draw_hair_settings_ppll(ui, config);
      } else {
        Self::draw_hair_settings_solid(ui, config);
      }
    }

    push_token.end();
  }

  fn draw_hair_settings_ppll(ui: &Ui, config: &mut Config) {
    next_widget_small(ui);
    ui.combo(
      "Hair display mode##ppll",
      &mut config.hair_ppll_display_mode,
      &[
        HairPPLLDisplayMode::Final,
        HairPPLLDisplayMode::Flat,
        HairPPLLDisplayMode::PpllOverlap,
        HairPPLLDisplayMode::Tangents,
        HairPPLLDisplayMode::Coverage,
      ],
      |idx| match *idx {
        HairPPLLDisplayMode::Flat => Cow::Borrowed("Flat"),
        HairPPLLDisplayMode::PpllOverlap => Cow::Borrowed("PPLL overlap"),
        HairPPLLDisplayMode::Tangents => Cow::Borrowed("Tangents"),
        HairPPLLDisplayMode::Coverage => Cow::Borrowed("Coverage"),
        _ => Cow::Borrowed("Final"),
      },
    );
  }

  fn draw_hair_settings_solid(ui: &Ui, config: &mut Config) {
    next_widget_small(ui);
    ui.combo(
      "Hair display mode##solid",
      &mut config.hair_solid_display_mode,
      &[
        HairSolidDisplayMode::Final,
        HairSolidDisplayMode::Flat,
        HairSolidDisplayMode::FollowGroups,
        HairSolidDisplayMode::Strands,
        HairSolidDisplayMode::RootTipPercentage,
      ],
      |idx| match *idx {
        HairSolidDisplayMode::Flat => Cow::Borrowed("Flat"),
        HairSolidDisplayMode::FollowGroups => Cow::Borrowed("Follow gr."),
        HairSolidDisplayMode::Strands => Cow::Borrowed("Strands"),
        HairSolidDisplayMode::RootTipPercentage => Cow::Borrowed("Root-tip %"),
        _ => Cow::Borrowed("Final"),
      },
    );
  }

  fn draw_hair_simulation_settings(ui: &Ui, config: &mut Config) {
    // TODO [LOW] flag to turn off collision? It can just set radii to 0.
    let push_token = ui.push_id("tressfx_sim");

    if ui.collapsing_header("Hair simulation", *HEADER_FLAGS) {
      if ui.button("Reset hair state") {
        config.reset_tfx_simulation_next_frame = true;
      }

      let sim: &mut TfxSimulation = &mut config.tfx_simulation;
      slider_small(ui, "Gravity", 0.0, 300.0, &mut sim.gravity);

      // Verlet integration
      slider_small(ui, "Damping", 0.0, 1.0, &mut sim.verlet_integration_damping);
      add_tooltip_to_previous_widget(ui, "Damping for verlet integration.\n0 - continue movement from previous frame\n1 - use only gravity and wind");

      // Wind
      // TODO [MEDIUM] add option to jitter direction/strength? Can be CPU only
      ui.text_disabled("Wind");
      slider_small(ui, "Wind strength", 0.0, 300.0, &mut sim.wind_strength);
      slider_position_phi(ui, "Wind position phi", &mut sim.wind_pos_phi);
      slider_position_theta(ui, "Wind position th", &mut sim.wind_pos_theta);

      // Global Shape Constraint
      ui.text_disabled("Global Shape Constraint");
      add_tooltip_to_previous_widget(
        ui,
        "Preserve initial shape of the hair.\nHappens every frame so this effect is VERY strong.",
      );
      slider_small(ui, "Stiffness##gsc", 0.0, 0.2, &mut sim.global_stiffness);
      slider_small(
        ui,
        "Strand range",
        0.0,
        1.0,
        &mut sim.global_stiffness_range,
      );
      add_tooltip_to_previous_widget(ui, "Which part of strand (whole or only near root) are affected by GSC.\n0 - only root is affected by GSC, so the tips will be 'bouncy'\n1 - whole strand is affected by GSC (less movement)",);

      // Local Shape Constraint
      ui.text_disabled("Local Shape Constraint");
      add_tooltip_to_previous_widget(
        ui,
        "(Local Shape Constraint)\nPreserve local shape of the hair (direction between consecutive vertices).\nUsed with e.g. curly hair.",
      );
      slider_small(ui, "Stiffness##lsc", 0.0, 1.0, &mut sim.local_stiffness);
      add_tooltip_to_previous_widget(
        ui,
        "0 - no local shape preservation (affected by gravity/wind more)\n1 - preserve relative vectors between vertices (less affected by forces)",
      );
      slider_small(
        ui,
        "Iterations##lsc",
        0,
        5,
        &mut sim.local_stiffness_iterations,
      );

      // length constraints
      ui.text_disabled("Length Constraint");
      add_tooltip_to_previous_widget(
        ui,
        "(Length Constraint)\nPreserve initial distance between strand vertices. Fix hair segments that are too long/short.",
      );
      slider_small(ui, "Stiffness##length", 0.0, 1.0, &mut sim.length_stiffness);
      slider_small(
        ui,
        "Iterations##length",
        0,
        5,
        &mut sim.length_constraint_iterations,
      );
    }

    push_token.end();
  }

  fn draw_entity(ui: &Ui, entity: &mut WorldEntity) {
    let push_token = ui.push_id(entity.name.clone());
    let material = &mut entity.material;

    let label = format!("Object: {}", entity.name);
    if ui.collapsing_header(label, *HEADER_FLAGS) {
      ui.text_disabled(format!(
        "Center: {}",
        vec3_to_pretty_str(entity.aabb.center())
      ));
      ui.text_disabled(format!(
        "Dimensions: {}",
        vec3_to_pretty_str(entity.aabb.dimensions())
      ));

      // material
      if material.specular_tex.is_none() {
        slider_small(ui, "Specular", 0.0, 1.0, &mut material.specular);
      }
      slider_small(ui, "Specular mul", 0.0, 5.0, &mut material.specular_mul);
      add_tooltip_to_previous_widget(ui, "Extra specular for eyes");

      // SSS
      ui.text_disabled("SSS forward pass");
      add_tooltip_to_previous_widget(ui, SSS_FORWARD_TOOLTIP);
      slider_small(
        ui,
        "SSS transluency",
        0.0,
        1.0,
        &mut material.sss_transluency,
      );
      add_tooltip_to_previous_widget(ui, "How much light passess through");
      slider_small(
        ui,
        "SSS width",
        SSSBlurPassCfg::SSS_WIDTH_MIN,
        SSSBlurPassCfg::SSS_WIDTH_MAX,
        &mut material.sss_width,
      );
      add_tooltip_to_previous_widget(ui, "Scale distance between light entrance and exit");
      slider_small(ui, "SSS bias", 0.0, 0.1, &mut material.sss_bias);
      add_tooltip_to_previous_widget(ui, "Similar to shadow maps, prevents 'acne' effect.");
      slider_small(ui, "SSS gain", 0.0, 1.0, &mut material.sss_gain);
      add_tooltip_to_previous_widget(ui, "Similar to shadow maps, prevents 'acne' effect.");
      slider_small(ui, "SSS strength", 0.0, 1.5, &mut material.sss_strength);
      add_tooltip_to_previous_widget(ui, "Scale the effect");
    }

    push_token.end();
  }

  fn draw_tfx_object(ui: &Ui, config: &mut Config, entity: &mut TfxObject) {
    let push_token = ui.push_id(entity.name.clone());
    let mat = &mut entity.material;

    let label = format!("TressFX: {}", entity.name);
    if ui.collapsing_header(label, *HEADER_FLAGS) {
      ui.text_disabled(format!("Strands: {}", entity.num_hair_strands));
      ui.text_disabled(format!(
        "Verts per strand: {}",
        entity.num_vertices_per_strand
      ));

      if ui.button("Toggle show collision meshes") {
        let is_currently_showing = config.debug_collision_sphere0.w != 0.0
          || config.debug_collision_sphere1.w != 0.0
          || config.debug_collision_sphere2.w != 0.0
          || config.debug_collision_sphere3.w != 0.0;
        if is_currently_showing {
          config.debug_collision_sphere0.w = 0.0;
          config.debug_collision_sphere1.w = 0.0;
          config.debug_collision_sphere2.w = 0.0;
          config.debug_collision_sphere3.w = 0.0;
        } else {
          let model_mat = entity.model_matrix.clone();
          let scale = entity.scale_debug_use_only;
          let cc_fn = |cc: Vec4| {
            let a = model_mat * cc;
            vec4(a.x, a.y, a.z, cc.w * scale)
          };
          config.debug_collision_sphere0 = cc_fn(entity.collision_capsule0);
          config.debug_collision_sphere1 = cc_fn(entity.collision_capsule1);
          config.debug_collision_sphere2 = cc_fn(entity.collision_capsule2);
          config.debug_collision_sphere3 = cc_fn(entity.collision_capsule3);
        }
      }

      slider_small(ui, "Radius", 0.001, 0.025, &mut entity.fiber_radius);
      add_tooltip_to_previous_widget(ui, "Radius of each strand");
      slider_small(ui, "Thin tip", 0.0, 1.0, &mut entity.thin_tip); // delta: 0.01,
      add_tooltip_to_previous_widget(ui, "Scale strand tip wrt to root");
      slider_small(
        ui,
        "Follow hairs",
        1,
        TfxObject::MAX_FOLLOW_HAIRS_PER_GUIDE,
        &mut entity.follow_hairs,
      );
      add_tooltip_to_previous_widget(ui, "Artificial strands around main, simulated strand");
      slider_small(
        ui,
        "Spread root",
        0.0,
        0.6,
        &mut entity.follow_hair_spread_root,
      );
      add_tooltip_to_previous_widget(ui, "Scatter follow strands at root");
      slider_small(
        ui,
        "Spread tip",
        0.0,
        0.6,
        &mut entity.follow_hair_spread_tip,
      );
      add_tooltip_to_previous_widget(ui, "Scatter follow strands at tip");

      // material
      let max_spec = 500.0;
      let min_shift = -0.1;
      let max_shift = 0.1;
      text_disabled_multiline(ui, "Kajiya-Kay hair shading model");
      color_rgb(ui, "Diffuse", &mut mat.albedo);
      slider_small(ui, "Opacity", 0.001, 1.0, &mut mat.opacity);

      color_rgb(ui, "Spec 1", &mut mat.specular_color1);
      slider_small(ui, "Spec exp 1", 0.0, max_spec, &mut mat.specular_power1);
      slider_small(ui, "Spec str 1", 0.0, 1.0, &mut mat.specular_strength1);
      slider_small(
        ui,
        "Spec shift 1",
        min_shift,
        max_shift,
        &mut mat.primary_shift,
      ); // delta 0.001;

      color_rgb(ui, "Spec 2", &mut mat.specular_color2);
      slider_small(ui, "Spec exp 2", 0.0, max_spec, &mut mat.specular_power2);
      slider_small(ui, "Spec str 2", 0.0, 1.0, &mut mat.specular_strength2);
      slider_small(
        ui,
        "Spec shift 2",
        min_shift,
        max_shift,
        &mut mat.secondary_shift,
      ); // delta 0.001;

      text_disabled_multiline(ui, "Ambient occlusion");
      slider_small(ui, "AO strength", 0.0, 1.0, &mut mat.ao_strength); // delta 0.01
      slider_small(ui, "AO exp", 0.0, 5.0, &mut mat.ao_exp); // delta 0.1
    }

    push_token.end();
  }

  fn draw_ambient_light(ui: &Ui, light: &mut LightAmbient) {
    let push_token = ui.push_id("ambient_light");

    if ui.collapsing_header("Ambient light", *HEADER_FLAGS) {
      color_rgb(ui, "Color", &mut light.color);
      slider_small(ui, "Energy", 0.0, 0.2, &mut light.energy);
    }

    push_token.end();
  }

  fn draw_light(ui: &Ui, name: &str, light: &mut LightCfg) {
    let push_token = ui.push_id(name);

    if ui.collapsing_header(name, *HEADER_FLAGS) {
      color_rgb(ui, "Color", &mut light.color);
      slider_small(ui, "Energy", 0.0, 5.0, &mut light.energy);
      slider_position_phi(ui, "Position phi", &mut light.pos_phi);
      slider_position_theta(ui, "Position th", &mut light.pos_theta);
      slider_small(ui, "Distance", 1.0, 20.0, &mut light.pos_distance);
    }

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
        slider_small(ui, "Blur radius", 0, 4, &mut shadows.blur_radius);
        slider_small(ui, "Hair blur radius", 0, 4, &mut shadows.blur_radius_tfx);
      }
      ui.slider("Strength", 0.0, 1.0, &mut shadows.strength);
      add_tooltip_to_previous_widget(ui, "Artificialy set maximal shadows strength");

      ui.slider("Bias", 0.001, 0.1, &mut shadows.bias);
      add_tooltip_to_previous_widget(ui, "Prevent shadow acne");
      ui.slider("Hair bias", 0.001, 0.1, &mut shadows.bias_hair_tfx);
      add_tooltip_to_previous_widget(ui, "Prevent shadow acne");
      slider_small(
        ui,
        "Hair radius mul",
        0.5,
        3.0,
        &mut shadows.hair_tfx_radius_multipler,
      );
      add_tooltip_to_previous_widget(ui, "Make hair strands thicker to cast bigger shadow");

      slider_position_phi(ui, "Position phi", &mut shadows.shadow_source.pos_phi);
      slider_position_theta(ui, "Position th", &mut shadows.shadow_source.pos_theta);
      // dir.add(this.cfg.shadows.directionalLight, 'posRadius', 1, 10).step(0.1).name('Position r');
    }

    push_token.end();
  }

  fn draw_sss_forward_pass(ui: &Ui, sss: &mut SSSForwardScatterPassCfg) {
    let push_token = ui.push_id("sss_forward_pass");

    if ui.collapsing_header("SSS forward pass", *HEADER_FLAGS) {
      text_disabled_multiline(
        ui,
        &(SSS_FORWARD_TOOLTIP.to_string() + ". See each object for more options."),
      );

      slider_position_phi(ui, "Position phi", &mut sss.source.pos_phi);
      slider_position_theta(ui, "Position th", &mut sss.source.pos_theta);
    }

    push_token.end();
  }

  fn draw_sss_blur(ui: &Ui, sss: &mut SSSBlurPassCfg) {
    let push_token = ui.push_id("sss_blur");

    if ui.collapsing_header("SSS blur", *HEADER_FLAGS) {
      text_disabled_multiline(ui, "Blur skin with special per-channel profile");
      slider_small(
        ui,
        "Blur width",
        SSSBlurPassCfg::SSS_WIDTH_MIN,
        SSSBlurPassCfg::SSS_WIDTH_MAX,
        &mut sss.blur_width,
      );
      add_tooltip_to_previous_widget(ui, "Distance in world units");
      slider_small(ui, "Blur strength", 0.0, 1.0, &mut sss.blur_strength);
      ui.checkbox("Blur follow surface", &mut sss.blur_follow_surface);
      add_tooltip_to_previous_widget(ui, "Slight changes for incident angles ~90dgr");
    }

    push_token.end();
  }

  fn draw_ssao(ui: &Ui, ssao: &mut SSAOConfig) {
    let push_token = ui.push_id("ssao");

    if ui.collapsing_header("SSAO", *HEADER_FLAGS) {
      slider_small(
        ui,
        "Kernel size",
        1,
        SSAOConfig::MAX_KERNEL_VALUES,
        &mut ssao.kernel_size,
      );
      slider_small(ui, "Radius", 0.1, 3.0, &mut ssao.radius);
      add_tooltip_to_previous_widget(ui, "SSAO max sample radius in world units");
      slider_small(ui, "Bias", 0.0, 0.1, &mut ssao.bias);
      slider_small(ui, "Blur radius", 0, 9, &mut ssao.blur_radius);
      add_tooltip_to_previous_widget(ui, "Blur radius in pixels");
      slider_small(ui, "Blur gauss sigma", 1.0, 6.0, &mut ssao.blur_gauss_sigma); // delta 0.1
      slider_small(
        ui,
        "Blur max depth diff",
        0.0,
        0.05,
        &mut ssao.blur_max_depth_distance,
      );
      add_tooltip_to_previous_widget(
        ui,
        "Discard samples during blur (based on linear depth 0-1)",
      );
      slider_small(ui, "AO strength", 0.0, 1.0, &mut ssao.ao_strength); // delta 0.01
      add_tooltip_to_previous_widget(ui, "Artificialy modify SSAO value");
      slider_small(ui, "AO exp", 0.0, 5.0, &mut ssao.ao_exp); // delta 0.1
      add_tooltip_to_previous_widget(ui, "Artificialy modify SSAO value");
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
        slider_small(ui, s.1, 0.0, 1.0, &mut s.0);
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
    color_rgb(ui, format!("##{}-color", label), &mut prop.color);
    ui.same_line();
    slider_small(ui, label, min, max, &mut prop.value);
  }

  fn draw_post_fx(ui: &Ui, postfx: &mut PostFxCfg) {
    let push_token = ui.push_id("PostFX");

    if ui.collapsing_header("PostFX", *HEADER_FLAGS) {
      ui.slider("Gamma", 1.0, 3.0, &mut postfx.gamma);
      ui.slider("Dither", 0.0, 2.0, &mut postfx.dither_strength);

      next_widget_small(ui);
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

      slider_small(ui, "Subpixel aa", 0.0, 2.0, &mut config.postfx.subpixel);
      add_tooltip_to_previous_widget(ui, "0.0 - off, 1.0 - 'soft' result, >1.0 - nonsense");

      slider_small(
        ui,
        "Relative Edge Treshold",
        0.063,
        0.333,
        &mut config.postfx.edge_threshold,
      );
      add_tooltip_to_previous_widget(
        ui,
        "The minimum amount of local contrast required to apply algorithm",
      );

      slider_small(
        ui,
        "Absolute Edge Treshold",
        0.0,
        0.0833,
        &mut config.postfx.edge_threshold_min,
      );
      add_tooltip_to_previous_widget(
        ui,
        "The minimum amount of contrast required to apply algorithm (mostly for dark areas)",
      );

      slider_small(
        ui,
        "Luma gamma",
        1.0,
        3.0,
        &mut config.postfx.fxaa_luma_gamma,
      );
      add_tooltip_to_previous_widget(
        ui,
        "FXAA uses luma to detect edges, which includes gamma correction to perceptual space",
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

fn text_disabled_multiline(ui: &Ui, text: &str) {
  let color = ui.style_color(StyleColor::TextDisabled);
  let style = ui.push_style_color(StyleColor::Text, color);
  ui.text_wrapped(text);
  style.end();
}

fn slider_small<T: AsRef<str>, K: DataTypeKind>(
  ui: &Ui,
  label: T,
  min: K,
  max: K,
  value: &mut K,
) -> bool {
  next_widget_small(ui);
  ui.slider(label, min, max, value)
}

fn slider_position_phi<T: AsRef<str>>(ui: &Ui, label: T, value: &mut f32) -> bool {
  slider_small(ui, label, -179.0, 179.0, value)
}

fn slider_position_theta<T: AsRef<str>>(ui: &Ui, label: T, value: &mut f32) -> bool {
  slider_small(ui, label, 15.0, 165.0, value)
}

pub fn color_rgb<T: AsRef<str>, K>(ui: &Ui, label: T, value: &mut K) -> bool
where
  K: Copy + Into<mint::Vector3<f32>> + From<mint::Vector3<f32>>,
{
  let flags = ColorEditFlags::NO_INPUTS
    | ColorEditFlags::NO_ALPHA
    | ColorEditFlags::NO_TOOLTIP
    | ColorEditFlags::INPUT_RGB
    | ColorEditFlags::DISPLAY_RGB;
  ui.color_edit3_config(label, value).flags(flags).build()
}
