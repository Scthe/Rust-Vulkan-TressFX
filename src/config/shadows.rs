use glam::{vec3, Vec3};

use crate::utils::spherical_to_cartesian_dgr;

pub struct ShadowLightProjection {
  pub left: f32,
  pub right: f32,
  pub top: f32,
  pub bottom: f32,
  pub near: f32,
  pub far: f32,
}

pub struct ShadowLightCfg {
  /// horizontal [dgr]
  pub pos_phi: f32,
  /// verical [dgr]
  pub pos_theta: f32,
  pub pos_distance: u32, // verify with projection box below!!!
  pub look_at_target: Vec3,
  pub projection: ShadowLightProjection,
}

pub struct ShadowsConfig {
  // TODO pub show_debug_view: bool,
  pub shadowmap_size: u32,
  pub use_pcss: bool,
  /// in pixels
  pub blur_radius: u32,
  pub bias: f32,
  /// in pixels
  pub blur_radius_tfx: u32,
  pub bias_hair_tfx: f32,
  pub hair_tfx_radius_multipler: f32,
  pub strength: f32,
  pub shadow_source: ShadowLightCfg,
}

impl ShadowsConfig {
  pub const SHADOWS_ORTHO_SIZE: u32 = 5;

  pub fn position(&self) -> Vec3 {
    // vec3(-10.0, 10.0, 10.0)
    // vec3(-10.0, 4.0, 0.0)
    spherical_to_cartesian_dgr(
      self.shadow_source.pos_phi,
      self.shadow_source.pos_theta,
      self.shadow_source.pos_distance as f32,
    )
  }
}

impl Default for ShadowsConfig {
  fn default() -> Self {
    let proj_box_side = ShadowsConfig::SHADOWS_ORTHO_SIZE as f32;
    Self {
      // show_debug_view: false,
      shadowmap_size: 1024 * 2,
      use_pcss: false,
      blur_radius: 0, // TODO 4,
      bias: 0.005,
      blur_radius_tfx: 1,
      bias_hair_tfx: 0.050,
      hair_tfx_radius_multipler: 1.1,
      strength: 0.7,
      shadow_source: ShadowLightCfg {
        pos_phi: 105.0,
        pos_theta: 45.0,
        pos_distance: ShadowsConfig::SHADOWS_ORTHO_SIZE,
        look_at_target: vec3(0.0, 2.0, 0.0),
        projection: ShadowLightProjection {
          left: -proj_box_side,
          right: proj_box_side,
          top: proj_box_side,
          bottom: -proj_box_side,
          near: 0.1,
          far: 20.0,
        },
      },
    }
  }
}
