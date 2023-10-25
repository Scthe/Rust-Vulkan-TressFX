use glam::{vec2, vec3, Vec2, Vec3};

pub struct CameraConfig {
  pub position: Vec3,
  pub rotation: Vec2,
  pub fov_dgr: f32,
  pub z_near: f32,
  pub z_far: f32,
}

impl Default for CameraConfig {
  fn default() -> Self {
    Self {
      position: vec3(4.0, 7.5, 9.0),
      // position: vec3(0.0, 2.5, 5.0), // en face
      // position: vec3(0, 3.5, 2), // closeup on hair
      rotation: vec2(-25f32, 0.0), // degrees
      // rotation: vec2(0.0, 0.0), // degrees
      fov_dgr: 75.0,
      z_near: 0.1,
      z_far: 100.0,
    }
  }
}
