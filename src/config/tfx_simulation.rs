use glam::Vec3;

use crate::utils::spherical_to_cartesian_dgr;

pub struct TfxSimulation {
  pub gravity: f32,
  pub verlet_integration_damping: f32,
  pub global_stiffness: f32,
  pub global_stiffness_range: f32,
  pub local_stiffness: f32,
  pub local_stiffness_iterations: u32,
  pub length_stiffness: f32,
  pub length_constraint_iterations: u32,

  // wind
  /// horizontal [dgr]
  pub wind_pos_phi: f32,
  /// verical [dgr]
  pub wind_pos_theta: f32,
  pub wind_strength: f32,
}

impl TfxSimulation {
  pub fn wind_position(&self) -> Vec3 {
    spherical_to_cartesian_dgr(self.wind_pos_phi, self.wind_pos_theta, 1.0)
  }
}

impl Default for TfxSimulation {
  fn default() -> Self {
    Self {
      gravity: 50.0,
      verlet_integration_damping: 0.5,
      // global
      global_stiffness: 0.1,
      global_stiffness_range: 0.2,
      // local
      local_stiffness: 0.15,
      local_stiffness_iterations: 1,
      // length
      length_stiffness: 0.95,
      length_constraint_iterations: 4,

      // wind
      wind_pos_phi: 140.0,
      wind_pos_theta: 105.0,
      wind_strength: 0.0,
    }
  }
}
