use glam::{const_vec3, vec3, Mat4, Vec3};

// const ROTATE_SENSITIVITY: f32 = 0.002;
const MOVE_SENSITIVITY: f32 = 0.05;
const WHEEL_SENSITIVITY: f32 = 0.2;
const VULKAN_UP: Vec3 = const_vec3!([0.0, 1.0, 0.0]);

pub struct CameraSettings {
  pub fov_dgr: f32,
  pub aspect_ratio: f32,
  pub z_near: f32,
  pub z_far: f32,
}
pub struct Camera {
  view_matrix: Mat4,
  perspective_matrix: Mat4,
  position: Vec3,
  // rotation_yaw: f32,
  // rotation_pitch: f32,
}

impl Camera {
  pub fn new(s: CameraSettings) -> Camera {
    let dst = 2.0;
    // let position = Vec3::new(-dst, dst, -dst);
    let position = Vec3::new(0f32, 0f32, -dst);

    Camera {
      position,
      // rotation_yaw: 0.0,
      // rotation_pitch: 0.0,
      // view_matrix: Mat4::from_translation(Vec3::ZERO),
      // view_matrix: Mat4::look_at_rh(position, Vec3::zero(), VULKAN_UP),
      view_matrix: Camera::recalc_view_matrix(position),
      // view_matrix: Mat4::identity(),
      // https://matthewwellings.com/blog/the-new-vulkan-coordinate-system/
      // https://www.saschawillems.de/blog/2019/03/29/flipping-the-vulkan-viewport/
      // though glam does have fixes already implemented
      perspective_matrix: Mat4::perspective_rh(
        s.fov_dgr.to_radians(),
        s.aspect_ratio,
        s.z_near,
        s.z_far,
      ),
      // perspective_matrix: Mat4::identity(),
    }
  }

  pub fn view_matrix(&self) -> &Mat4 {
    &self.view_matrix
  }

  pub fn perspective_matrix(&self) -> &Mat4 {
    &self.perspective_matrix
  }

  /*
  pub fn rotate_yaw_pitch(&mut self, delta_yaw: f32, delta_pitch: f32) {
    self.rotation_yaw += delta_yaw * ROTATE_SENSITIVITY;
    self.rotation_pitch += delta_pitch * ROTATE_SENSITIVITY;
    let safe_pi = (PI / 2) * 0.95; // no extremes pls! limit 90dgr up down to only [-85, 85]
    self.rotation_pitch = self.rotation_pitch.clamp(-safe_pi, safe_pi);
  }
  */

  pub fn move_forward(&mut self, delta: f32) {
    let dvec = vec3(0f32, 0f32, delta * WHEEL_SENSITIVITY);
    self.apply_move(dvec);
  }

  // `move` is a reserved keyword..
  pub fn move_(&mut self, delta: Vec3) {
    self.apply_move(delta * MOVE_SENSITIVITY);
  }

  // TODO multiply by delta time?
  fn apply_move(&mut self, delta: Vec3) {
    self.position = self.position + delta;
    self.view_matrix = Camera::recalc_view_matrix(self.position);
  }

  // fn recalc_view_matrix(position: Vec3, yaw: f32, pitch: f32) -> Mat4 {
  fn recalc_view_matrix(position: Vec3) -> Mat4 {
    // https://github.com/h3r2tic/dolly/blob/main/src/drivers/yaw_pitch.rs#L83 ?
    Mat4::look_at_rh(position, Vec3::zero(), VULKAN_UP)
  }
}
