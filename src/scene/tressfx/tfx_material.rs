use glam::Vec3;

use crate::utils::color_hex_to_vec;

pub struct TfxMaterial {
  pub albedo: Vec3,
  pub ao_strength: f32,
  pub ao_exp: f32,
  //
  pub specular_color1: Vec3,
  pub specular_power1: f32,
  pub specular_strength1: f32,
  pub primary_shift: f32,
  //
  pub specular_color2: Vec3,
  pub specular_power2: f32,
  pub specular_strength2: f32,
  pub secondary_shift: f32,
}

impl Default for TfxMaterial {
  fn default() -> Self {
    Self {
      albedo: color_hex_to_vec(31, 26, 24),
      ao_strength: 1.0,
      ao_exp: 3.1,
      specular_color1: color_hex_to_vec(87, 43, 24),
      specular_power1: 160.0,
      specular_strength1: 0.27,
      primary_shift: 0.005,
      specular_color2: color_hex_to_vec(138, 129, 111),
      specular_power2: 400.0,
      specular_strength2: 0.07,
      secondary_shift: -0.06,
    }
  }
}
