use bytemuck;
use glam::{Vec2, Vec3, Vec4};

use crate::{scene::Camera, vk_ctx::VkCtx};

/// Global config data, updated per-frame
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct GlobalConfigUBO {
  pub u_camera_position: Vec3,
  pub u_viewport: Vec2,
  // ao
  // float u_aoStrength;
  // float u_aoExp;
  // Shadow
  // vec4 u_directionalShadowCasterPosition; // [position.xyz, bias (negative if pcss)]
  // int u_directionalShadowSampleRadius;
  // float u_maxShadowContribution;
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
}

unsafe impl bytemuck::Zeroable for GlobalConfigUBO {}
unsafe impl bytemuck::Pod for GlobalConfigUBO {}

impl GlobalConfigUBO {
  pub fn new(camera: &Camera, vk_app: &VkCtx) -> GlobalConfigUBO {
    let vp = vk_app.window_size();
    GlobalConfigUBO {
      u_camera_position: camera.position(),
      u_viewport: Vec2::new(vp.width as f32, vp.height as f32),
      u_light_ambient: Vec4::new(1.0, 1.0, 1.0, 0.1),
      u_light0_position: Vec3::new(10.0, 10.0, 10.0),
      u_light0_color: Vec4::new(1.0, 1.0, 1.0, 0.8),
      u_light1_position: Vec3::new(-10.0, -5.0, 10.0),
      u_light1_color: Vec4::new(0.7, 0.7, 1.0, 0.5),
      u_light2_position: Vec3::new(-5.0, 2.0, -10.0),
      u_light2_color: Vec4::new(1.0, 0.4, 0.4, 0.7),
    }
  }
}
