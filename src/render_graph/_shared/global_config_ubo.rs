use bytemuck;
use glam::{Mat4, Vec2, Vec3, Vec4};

/// Global config data, updated per-frame
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct GlobalConfigUBO {
  /// model view projection matrix for current camera
  /// Shared for all models, so might as well
  /// TODO split this from config
  pub u_mvp: Mat4,
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
