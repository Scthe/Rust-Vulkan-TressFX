use ash::vk;
use glam::{vec3, vec4, Mat4, Vec3, Vec4Swizzles};

use crate::config::Config;

const ROTATE_SENSITIVITY: f32 = 0.002;
const MOVE_SENSITIVITY: f32 = 0.3;
const WHEEL_SENSITIVITY: f32 = 0.2;

pub struct Camera {
  view_matrix: Mat4,
  perspective_matrix: Mat4,
  position: Vec3,
  /// in radians
  rotation_yaw: f32,
  /// in radians
  rotation_pitch: f32,
}

impl Camera {
  pub fn new(config: &Config, window_size: vk::Extent2D) -> Camera {
    let cam_cfg = &config.camera;
    let position = cam_cfg.position;
    let rotation_yaw = cam_cfg.rotation.x.to_radians();
    let rotation_pitch = cam_cfg.rotation.y.to_radians();
    let aspect_ratio: f32 = window_size.width as f32 / window_size.height as f32;

    Camera {
      position,
      rotation_yaw,
      rotation_pitch,
      view_matrix: calc_view_matrix(position, rotation_yaw, rotation_pitch),
      // https://matthewwellings.com/blog/the-new-vulkan-coordinate-system/
      // https://www.saschawillems.de/blog/2019/03/29/flipping-the-vulkan-viewport/
      // though glam does have fixes already implemented
      perspective_matrix: Mat4::perspective_rh(
        cam_cfg.fov_dgr.to_radians(),
        aspect_ratio,
        cam_cfg.z_near,
        cam_cfg.z_far,
      ),
    }
  }

  pub fn position(&self) -> Vec3 {
    self.position.clone()
  }

  pub fn view_matrix(&self) -> &Mat4 {
    &self.view_matrix
  }

  pub fn perspective_matrix(&self) -> &Mat4 {
    &self.perspective_matrix
  }

  #[allow(dead_code)]
  pub fn view_projection_matrix(&self) -> Mat4 {
    let v = self.view_matrix();
    let p = self.perspective_matrix();
    p.mul_mat4(&v)
  }

  pub fn model_view_projection_matrix(&self, model_matrix: Mat4) -> Mat4 {
    let v = self.view_matrix();
    let p = self.perspective_matrix();
    let m = model_matrix;
    Self::calc_model_view_projection_matrix(&m, v, p)
  }

  pub fn calc_model_view_projection_matrix(
    model_matrix: &Mat4,
    view_matrix: &Mat4,
    projection_matrix: &Mat4,
  ) -> Mat4 {
    projection_matrix
      .mul_mat4(&view_matrix)
      .mul_mat4(&model_matrix)
  }

  pub fn rotate_yaw_pitch(&mut self, delta_yaw: f32, delta_pitch: f32) {
    self.rotation_yaw += delta_yaw * ROTATE_SENSITIVITY;
    self.rotation_pitch += delta_pitch * ROTATE_SENSITIVITY;
    let safe_pi = (std::f32::consts::PI / 2.0) * 0.95; // no extremes pls! limit 90dgr up down to only [-85, 85]
    self.rotation_pitch = self.rotation_pitch.clamp(-safe_pi, safe_pi);

    self.update_view_matrix();
  }

  pub fn move_forward(&mut self, delta: f32) {
    let dvec = vec3(0f32, 0f32, delta * WHEEL_SENSITIVITY);
    self.apply_move(dvec);
  }

  // `move` is a reserved keyword..
  pub fn move_(&mut self, delta: Vec3) {
    self.apply_move(delta * MOVE_SENSITIVITY);
  }

  /// TODO [LOW] multiply by delta time?
  /// - `delta` is in camera local space
  fn apply_move(&mut self, delta: Vec3) {
    let mut mat_rot = calc_rotation_matrix(self.rotation_yaw, self.rotation_pitch);
    // invert as we have to revert to get proper vectors in rows
    // tbh we could revert order of mat4*vec4 into vec4*mat4 if lib supports it
    mat_rot = mat_rot.transpose();

    let delta_global = mat_rot * vec4(delta.x, delta.y, delta.z, 1.0);
    self.position = self.position + delta_global.xyz();
    /*trace!(
      "apply_move(old_position={}, Δlocal={})  --- Δglobal={} ----> position={}",
      old_position,
      delta,
      delta_global,
      self.position
    );*/
    self.update_view_matrix();
  }

  fn update_view_matrix(&mut self) {
    self.view_matrix = calc_view_matrix(self.position, self.rotation_yaw, self.rotation_pitch);
  }

  /// Mostly for debug
  /// - returns `(side, up, forward)`
  pub fn get_rotation_axes(&self) -> (Vec3, Vec3, Vec3) {
    let mut mat_rot = calc_rotation_matrix(self.rotation_yaw, self.rotation_pitch);
    mat_rot = mat_rot.transpose();
    (
      (mat_rot * vec4(1.0, 0.0, 0.0, 1.0)).xyz(),
      (mat_rot * vec4(0.0, 1.0, 0.0, 1.0)).xyz(),
      (mat_rot * vec4(0.0, 0.0, 1.0, 1.0)).xyz(),
    )
  }
}

fn calc_view_matrix(position: Vec3, yaw: f32, pitch: f32) -> Mat4 {
  let mat_rot = calc_rotation_matrix(yaw, pitch);

  // we have to reverse position, as moving camera X units
  // moves scene -X units
  let mat_tra = Mat4::from_translation(-position);

  mat_rot * mat_tra
}

fn calc_rotation_matrix(yaw: f32, pitch: f32) -> Mat4 {
  let mat_p = Mat4::from_rotation_x(pitch);
  let mat_y = Mat4::from_rotation_y(yaw);
  mat_p * mat_y
}
