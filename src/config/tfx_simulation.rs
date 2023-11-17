use glam::Vec3;

use crate::utils::spherical_to_cartesian_dgr;

// TODO [HIGH] Show wind for debug
pub struct TfxSimulation {
  pub gravity: f32,
  pub verlet_integration_damping: f32,
  pub global_stiffness: f32,
  pub global_stiffness_range: f32,
  pub local_stiffness: f32,
  // vsp_force, vsp_threshold
  pub length_constraint_iterations: u32,
  pub length_stiffness: f32,

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
      gravity: 0.0,
      verlet_integration_damping: 1.0,
      global_stiffness: 0.05,
      global_stiffness_range: 0.3,
      local_stiffness: 0.9,
      length_constraint_iterations: 1,
      length_stiffness: 1.0,

      // wind
      wind_pos_phi: 140.0,
      wind_pos_theta: 105.0,
      wind_strength: 0.0,
    }
  }
}
