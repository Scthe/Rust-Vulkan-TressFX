use bytemuck;
use glam::{vec4, Mat4, Vec3, Vec4};

use crate::{
  config::{
    spherical_to_cartesian_dgr, ColorGradingProp, Config, LightAmbient, LightCfg, SSAOConfig,
  },
  scene::Camera,
  vk_ctx::VkCtx,
};

/// Global config data, updated per-frame
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct GlobalConfigUBO {
  pub u_camera_position: Vec3,
  pub u_viewport_and_near_far: Vec4,
  pub u_view_mat: Mat4,
  pub u_projection: Mat4,
  pub u_inv_projection_mat: Mat4, // inverse projection matrix
  // Shadow
  // vec4 u_directionalShadowCasterPosition; // [position.xyz, bias (negative if pcss)]
  // int u_directionalShadowSampleRadius;
  pub u_ao_and_shadow_contrib: Vec4,
  // sss
  // float u_sssFarPlane;
  // mat4 u_sssMatrix_VP;
  // Lights
  pub u_light_ambient: Vec4,
  pub u_light0_position: Vec3,
  pub u_light0_color: Vec4,
  pub u_light1_position: Vec3,
  pub u_light1_color: Vec4,
  pub u_light2_position: Vec3,
  pub u_light2_color: Vec4,
  // SSAO
  pub u_ssao: Vec4,
  pub u_ssao2: Vec4,
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

impl GlobalConfigUBO {
  pub fn new(vk_app: &VkCtx, config: &Config, camera: &Camera) -> GlobalConfigUBO {
    let vp = vk_app.window_size();
    let cam_cfg = &config.camera;
    let postfx = &config.postfx;
    let color_grading = &postfx.color_grading;
    let ssao_vp = config.get_ssao_viewport_size();

    GlobalConfigUBO {
      u_camera_position: camera.position(),
      u_viewport_and_near_far: vec4(
        vp.width as f32,
        vp.height as f32,
        cam_cfg.z_near,
        cam_cfg.z_far,
      ),
      u_view_mat: *camera.view_matrix(),
      u_projection: *camera.perspective_matrix(),
      u_inv_projection_mat: camera.perspective_matrix().inverse(),
      u_ao_and_shadow_contrib: Vec4::new(0.0, 0.0, config.shadows.strength, 1.0),
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
      u_ssao2: vec4(config.ssao.kernel_size as f32, 0.0, 0.0, 0.0),
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
  vec4(light.color[0], light.color[1], light.color[2], light.energy)
}

fn light_color(light: &LightCfg) -> Vec4 {
  vec4(light.color[0], light.color[1], light.color[2], light.energy)
}

fn light_pos(light: &LightCfg) -> Vec3 {
  spherical_to_cartesian_dgr(light.pos_phi, light.pos_theta, light.pos_distance)
}

fn pack_color_grading_prop(prop: &ColorGradingProp) -> Vec4 {
  vec4(prop.color.x, prop.color.y, prop.color.z, prop.value)
}
