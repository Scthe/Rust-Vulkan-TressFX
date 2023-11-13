use bytemuck;
use glam::{vec4, Mat4, Vec3, Vec4};

use crate::{
  config::{ColorGradingProp, Config, LightAmbient, LightCfg, SSAOConfig},
  render_graph::{
    shadow_map_pass::ShadowMapPass, sss_depth_pass::SSSDepthPass, tfx_render::TfxPpllBuildPass,
  },
  scene::Camera,
  utils::{into_vec4, mint3_into_vec4, spherical_to_cartesian_dgr},
  vk_ctx::VkCtx,
};

/// Global config data, updated per-frame
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct GlobalConfigUBO {
  pub u_camera_position_and_display_mode: Vec4,
  pub u_viewport_and_near_far: Vec4,
  pub u_view_mat: Mat4,
  pub u_projection_mat: Mat4,
  pub u_inv_projection_mat: Mat4, // inverse projection matrix
  pub u_view_projection_mat: Mat4,
  pub u_tfx_hair_settings: Vec4, // [hairDisplayMode, u_tfxLinkedListPoolSize, -, -]
  // AO + Shadow
  pub u_shadow_matrix_vp: Mat4,
  pub u_shadow_radius_and_bias: Vec4, // [u_shadowRadiusForwardShading, u_shadowBiasForwardShading, u_shadowRadiusTfx, u_shadowBiasTfx]
  pub u_shadow_caster_position: Vec4, // [position.xyz, u_shadowsTechnique]
  pub u_ao_settings: Vec4, // (u_aoStrength, u_aoExp, showDebugPositions+u_maxShadowContribution, -)
  // sss
  pub u_sss_settings: Vec4, // [u_sssPosition, u_sssFarPlane]
  pub u_sss_matrix_vp: Mat4,
  pub u_sss_blur: Vec4, // [u_sssWidth, u_sssStrength, u_sssFovy+u_sssFollowSurface, -]
  // Lights
  pub u_light_ambient: Vec4,
  pub u_light0_position: Vec3,
  pub u_light0_color: Vec4,
  pub u_light1_position: Vec3,
  pub u_light1_color: Vec4,
  pub u_light2_position: Vec3,
  pub u_light2_color: Vec4,
  // SSAO
  pub u_ssao: Vec4,  // [u_noiseScale.xy, u_radius, u_bias]
  pub u_ssao2: Vec4, // [u_kernelSize, u_linear_depth_preview_range.yz, -]
  // FXAA
  pub u_fxaa_settings: Vec4,
  // Color correction
  pub u_tonemapping: Vec4,
  pub u_tonemapping2: Vec4,
  // TONEMAPPING
  pub u_color_saturation: Vec4, // general
  pub u_color_contrast: Vec4,
  pub u_color_gamma: Vec4,
  pub u_color_gain: Vec4,
  pub u_color_offset: Vec4,
  pub u_color_saturation_shadows: Vec4, // shadows
  pub u_color_contrast_shadows: Vec4,
  pub u_color_gamma_shadows: Vec4,
  pub u_color_gain_shadows: Vec4,
  pub u_color_offset_shadows: Vec4,
  pub u_color_saturation_midtones: Vec4, // midtones
  pub u_color_contrast_midtones: Vec4,
  pub u_color_gamma_midtones: Vec4,
  pub u_color_gain_midtones: Vec4,
  pub u_color_offset_midtones: Vec4,
  pub u_color_saturation_highlights: Vec4, // highlights
  pub u_color_contrast_highlights: Vec4,
  pub u_color_gamma_highlights: Vec4,
  pub u_color_gain_highlights: Vec4,
  pub u_color_offset_highlights: Vec4,
}

unsafe impl bytemuck::Zeroable for GlobalConfigUBO {}
unsafe impl bytemuck::Pod for GlobalConfigUBO {}

/// Result negative if flag is true. Result is positive if flag is false.
/// Use only with values that can be >0!
fn encode_flag_in_value_sign(flag: bool, value: f32) -> f32 {
  let sign = if flag { -1.0 } else { 1.0 };
  value * sign
}

impl GlobalConfigUBO {
  pub fn new(vk_app: &VkCtx, config: &Config, camera: &Camera) -> GlobalConfigUBO {
    let vp = vk_app.window_size();
    let cam_cfg = &config.camera;
    let cam_pos = camera.position();
    let postfx = &config.postfx;
    let color_grading = &postfx.color_grading;
    let shadows = &config.shadows;
    let shadow_pos = shadows.shadow_source.position();
    let sss_frw = &config.sss_forward_scatter;
    let sss_frw_pos = sss_frw.source.position();
    let ssao_vp = config.get_ssao_viewport_size();

    GlobalConfigUBO {
      u_camera_position_and_display_mode: vec4(
        cam_pos.x,
        cam_pos.y,
        cam_pos.z,
        config.display_mode as _,
      ),
      u_viewport_and_near_far: vec4(
        vp.width as f32,
        vp.height as f32,
        cam_cfg.z_near,
        cam_cfg.z_far,
      ),
      u_view_mat: *camera.view_matrix(),
      u_projection_mat: *camera.perspective_matrix(),
      u_inv_projection_mat: camera.perspective_matrix().inverse(),
      u_view_projection_mat: camera.view_projection_matrix(),
      u_tfx_hair_settings: vec4(
        config.get_hair_display_mode() as f32,
        TfxPpllBuildPass::get_ppll_data_nodes_count(config.get_viewport_size()) as f32,
        0.0,
        0.0,
      ),
      // shadows:
      u_shadow_matrix_vp: ShadowMapPass::get_light_shadow_mvp(
        &shadows.shadow_source,
        Mat4::IDENTITY,
      ),
      u_shadow_radius_and_bias: vec4(
        shadows.blur_radius as _,
        shadows.bias,
        shadows.blur_radius_tfx as _,
        shadows.bias_hair_tfx,
      ),
      u_shadow_caster_position: into_vec4(shadow_pos, shadows.shadow_technique as _),
      u_ao_settings: Vec4::new(
        config.ssao.ao_strength,
        config.ssao.ao_exp,
        encode_flag_in_value_sign(config.show_debug_positions, shadows.strength),
        0.0,
      ),
      // sss
      u_sss_settings: vec4(
        sss_frw_pos.x,
        sss_frw_pos.y,
        sss_frw_pos.z,
        sss_frw.source.projection.far,
      ),
      u_sss_matrix_vp: SSSDepthPass::get_sss_forward_mvp(&sss_frw.source, Mat4::IDENTITY),
      u_sss_blur: vec4(
        config.sss_blur.blur_width,
        config.sss_blur.blur_strength,
        encode_flag_in_value_sign(
          config.sss_blur.blur_follow_surface,
          config.get_camera_fov_y(),
        ),
        0.0,
      ),
      // lights
      u_light_ambient: light_ambient(&config.light_ambient),
      u_light0_position: light_pos(&config.light0),
      u_light0_color: light_color(&config.light0),
      u_light1_position: light_pos(&config.light1),
      u_light1_color: light_color(&config.light1),
      u_light2_position: light_pos(&config.light2),
      u_light2_color: light_color(&config.light2),
      // SSAO
      u_ssao: vec4(
        (ssao_vp.width / SSAOConfig::RNG_VECTOR_TEXTURE_SIZE) as _,
        (ssao_vp.height / SSAOConfig::RNG_VECTOR_TEXTURE_SIZE) as _,
        config.ssao.radius,
        config.ssao.bias,
      ),
      u_ssao2: vec4(
        config.ssao.kernel_size as f32,
        // values are negative!
        config.linear_depth_preview_range.max_element(), // near
        config.linear_depth_preview_range.min_element(), // far
        0.0,
      ),
      // FXAA
      u_fxaa_settings: vec4(
        postfx.subpixel,
        config.fxaa_edge_threshold(),
        postfx.edge_threshold_min,
        postfx.fxaa_luma_gamma,
      ),
      // Color correction
      u_tonemapping: vec4(
        postfx.exposure,
        postfx.white_point,
        postfx.aces_c,
        postfx.aces_s,
      ),
      u_tonemapping2: vec4(
        postfx.dither_strength,
        postfx.tonemapping_op as f32,
        color_grading.shadows_max,
        color_grading.highlights_min,
      ),
      // TONEMAPPING
      u_color_saturation: pack_color_grading_prop(&color_grading.global.saturation), // general
      u_color_contrast: pack_color_grading_prop(&color_grading.global.contrast),
      u_color_gamma: pack_color_grading_prop(&color_grading.global.gamma),
      u_color_gain: pack_color_grading_prop(&color_grading.global.gain),
      u_color_offset: pack_color_grading_prop(&color_grading.global.offset),
      u_color_saturation_shadows: pack_color_grading_prop(&color_grading.shadows.saturation), // shadows
      u_color_contrast_shadows: pack_color_grading_prop(&color_grading.shadows.contrast),
      u_color_gamma_shadows: pack_color_grading_prop(&color_grading.shadows.gamma),
      u_color_gain_shadows: pack_color_grading_prop(&color_grading.shadows.gain),
      u_color_offset_shadows: pack_color_grading_prop(&color_grading.shadows.offset),
      u_color_saturation_midtones: pack_color_grading_prop(&color_grading.midtones.saturation), // midtones
      u_color_contrast_midtones: pack_color_grading_prop(&color_grading.midtones.contrast),
      u_color_gamma_midtones: pack_color_grading_prop(&color_grading.midtones.gamma),
      u_color_gain_midtones: pack_color_grading_prop(&color_grading.midtones.gain),
      u_color_offset_midtones: pack_color_grading_prop(&color_grading.midtones.offset),
      u_color_saturation_highlights: pack_color_grading_prop(&color_grading.highlights.saturation), // highlights
      u_color_contrast_highlights: pack_color_grading_prop(&color_grading.highlights.contrast),
      u_color_gamma_highlights: pack_color_grading_prop(&color_grading.highlights.gamma),
      u_color_gain_highlights: pack_color_grading_prop(&color_grading.highlights.gain),
      u_color_offset_highlights: pack_color_grading_prop(&color_grading.highlights.offset),
    }
  }
}

fn light_ambient(light: &LightAmbient) -> Vec4 {
  mint3_into_vec4(light.color, light.energy)
}

fn light_color(light: &LightCfg) -> Vec4 {
  mint3_into_vec4(light.color, light.energy)
}

fn light_pos(light: &LightCfg) -> Vec3 {
  spherical_to_cartesian_dgr(light.pos_phi, light.pos_theta, light.pos_distance)
}

fn pack_color_grading_prop(prop: &ColorGradingProp) -> Vec4 {
  mint3_into_vec4(prop.color, prop.value)
}
