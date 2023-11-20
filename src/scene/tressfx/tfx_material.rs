use mint::Vector3;

use crate::utils::{color_hex_to_vec, vec3_to_mint};

/// Kajiya-Kay hair shading model
///
/// ### References
/// http://developer.amd.com/wordpress/media/2012/10/Scheuermann_HairRendering.pdf
/// http://www.cemyuksel.com/courses/conferences/siggraph2010-hair/S2010_HairCourseNotes-Chapter4.pdf
pub struct TfxMaterial {
  pub albedo: Vector3<f32>,
  pub opacity: f32,
  pub ao_strength: f32,
  pub ao_exp: f32,
  //
  pub specular_color1: Vector3<f32>,
  pub specular_power1: f32,
  pub specular_strength1: f32,
  pub primary_shift: f32,
  //
  pub specular_color2: Vector3<f32>,
  pub specular_power2: f32,
  pub specular_strength2: f32,
  pub secondary_shift: f32,
}

impl Default for TfxMaterial {
  fn default() -> Self {
    Self {
      albedo: vec3_to_mint(color_hex_to_vec(23, 15, 12)),
      opacity: 0.9,
      ao_strength: 1.0,
      ao_exp: 3.1,
      specular_color1: vec3_to_mint(color_hex_to_vec(104, 67, 44)),
      specular_power1: 20.0,
      specular_strength1: 0.075,
      primary_shift: 0.005,
      specular_color2: vec3_to_mint(color_hex_to_vec(138, 129, 111)),
      specular_power2: 380.0,
      specular_strength2: 0.23,
      secondary_shift: -0.06,
    }
  }
}
