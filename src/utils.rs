use glam::{vec3, Vec3};
use rand::rngs::ThreadRng;
use rand::Rng;

/// Convert u8 [0..255) into float
pub fn color_u8_to_float(col_u8: u8) -> f32 {
  (col_u8 as f32) / 255.0
}

/// Convert u8 [0..255) into float vector
pub fn color_hex_to_vec(c0: u8, c1: u8, c2: u8) -> Vec3 {
  vec3(
    color_u8_to_float(c0),
    color_u8_to_float(c1),
    color_u8_to_float(c2),
  )
}

/// Convert spherical->cartesian. Both angles in degrees.
pub fn spherical_to_cartesian_dgr(phi_dgr: f32, theta_dgr: f32, distance: f32) -> Vec3 {
  spherical_to_cartesian_rad(phi_dgr.to_radians(), theta_dgr.to_radians(), distance)
}

/// Convert spherical->cartesian. Both angles in radians.
pub fn spherical_to_cartesian_rad(phi: f32, theta: f32, distance: f32) -> Vec3 {
  vec3(
    f32::cos(phi) * f32::sin(theta) * distance,
    f32::cos(theta) * distance,
    f32::sin(phi) * f32::sin(theta) * distance,
  )
}

/// https://registry.khronos.org/OpenGL-Refpages/gl4/html/mix.xhtml
pub fn lerp_f32(min: f32, max: f32, weight: f32) -> f32 {
  min + (max - min) * weight
}

pub fn vec3_to_pretty_str(v: Vec3) -> String {
  format!("[{:.1}, {:.1}, {:.1}]", v.x, v.y, v.z)
}

pub fn vec3_to_mint(v: Vec3) -> mint::Vector3<f32> {
  mint::Vector3::from_slice(v.as_ref())
}

/// Generate random vectors
pub struct RngVectorGenerator {
  rng: ThreadRng,
}

impl RngVectorGenerator {
  pub fn new() -> Self {
    Self {
      rng: rand::thread_rng(),
    }
  }

  /// Get random direction in hemisphere.
  /// Not exactly uniform distribution, but meh..
  pub fn generate_rng_hemisphere_vector(&mut self) -> Vec3 {
    let tmp = glam::vec3(
      self.rng.gen::<f32>() * 2.0 - 1.0, // [-1, 1]
      self.rng.gen::<f32>() * 2.0 - 1.0, // [-1, 1]
      self.rng.gen::<f32>(),             // [0, 1], HEMIsphere, not full sphere
    );
    tmp.normalize()
  }

  /// Similar to `generate_rng_hemisphere_vector`, but it will be points
  /// inside hemisphere. Points will be `weighted` toward away from the center.
  pub fn generate_rng_hemisphere_point(&mut self, weight: f32) -> Vec3 {
    let dir = self.generate_rng_hemisphere_vector();

    // get_random_point_in_hemisphere
    // ATM points lie on edge of sphere, randomize then inside
    let scale_fac = lerp_f32(0.1, 1.0, weight * weight);
    dir * scale_fac
  }
}
