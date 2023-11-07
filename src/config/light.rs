use mint::Vector3;

use crate::utils::{color_hex_to_vec, vec3_to_mint};

pub struct LightAmbient {
  pub color: Vector3<f32>,
  pub energy: f32,
}

impl Default for LightAmbient {
  fn default() -> Self {
    Self {
      color: vec3_to_mint(color_hex_to_vec(160, 160, 160)),
      energy: 0.02,
    }
  }
}

pub struct LightCfg {
  /// horizontal [dgr]
  pub pos_phi: f32,
  /// verical [dgr]
  pub pos_theta: f32,
  pub pos_distance: f32,
  pub color: Vector3<f32>,
  pub energy: f32,
}

impl LightCfg {
  pub fn light0() -> Self {
    LightCfg {
      pos_phi: 125.0,  // horizontal [dgr]
      pos_theta: 45.0, // verical [dgr]
      pos_distance: 10.0,
      color: vec3_to_mint(color_hex_to_vec(214, 197, 208)),
      energy: 1.0,
    }
  }

  pub fn light1() -> Self {
    LightCfg {
      pos_phi: 45.0,   // horizontal [dgr]
      pos_theta: 82.0, // verical [dgr]
      pos_distance: 10.0,
      color: vec3_to_mint(color_hex_to_vec(214, 166, 166)),
      energy: 0.80,
    }
  }

  pub fn light2() -> Self {
    LightCfg {
      pos_phi: -105.0, // horizontal [dgr]
      pos_theta: 55.0, // verical [dgr]
      pos_distance: 10.0,
      color: vec3_to_mint(color_hex_to_vec(133, 171, 169)),
      energy: 0.55,
    }
  }
}
