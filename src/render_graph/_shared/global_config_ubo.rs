use bytemuck;
use glam::{vec3, vec4, Vec3, Vec4};

use crate::{
  config::{spherical_to_cartesian_dgr, Config, LightAmbient, LightCfg},
  scene::Camera,
  vk_ctx::VkCtx,
};

/// Global config data, updated per-frame
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct GlobalConfigUBO {
  pub u_camera_position: Vec3,
  pub u_viewport_and_near_far: Vec4,
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
  // FXAA
  pub u_fxaa_settings: Vec3,
}

unsafe impl bytemuck::Zeroable for GlobalConfigUBO {}
unsafe impl bytemuck::Pod for GlobalConfigUBO {}

impl GlobalConfigUBO {
  pub fn new(vk_app: &VkCtx, config: &Config, camera: &Camera) -> GlobalConfigUBO {
    let vp = vk_app.window_size();
    let cam_cfg = &config.camera;
    let postfx = &config.postfx;

    GlobalConfigUBO {
      u_camera_position: camera.position(),
      u_viewport_and_near_far: vec4(
        vp.width as f32,
        vp.height as f32,
        cam_cfg.z_near,
        cam_cfg.z_far,
      ),
      u_ao_and_shadow_contrib: Vec4::new(0.0, 0.0, config.shadows.strength, 1.0),
      // lights
      u_light_ambient: light_ambient(&config.light_ambient),
      u_light0_position: light_pos(&config.light0),
      u_light0_color: light_color(&config.light0),
      u_light1_position: light_pos(&config.light1),
      u_light1_color: light_color(&config.light1),
      u_light2_position: light_pos(&config.light2),
      u_light2_color: light_color(&config.light2),
      // FXAA
      u_fxaa_settings: vec3(
        postfx.subpixel,
        config.fxaa_edge_threshold(),
        postfx.edge_threshold_min,
      ),
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
