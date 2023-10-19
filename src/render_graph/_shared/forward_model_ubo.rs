use bytemuck;
use glam::Mat4;

use crate::scene::{Camera, WorldEntity};

// All below must match shader-defined consts
const FLAG_IS_METALIC: i32 = 1;
const FLAG_USE_SPECULAR_TEXTURE: i32 = 2;
const FLAG_USE_HAIR_SHADOW_TEXTURE: i32 = 4;

// add mvp here? If we add ui, we have to reupload anyway cuz materials can change
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct ForwardModelUBO {
  pub u_model_matrix: Mat4,
  /// model view projection matrix for current camera
  pub u_model_view_projection_matrix: Mat4,
  // material
  pub u_specular: f32,
  pub u_specular_mul: f32,
  pub u_material_flags: i32,
  pub u_sss_transluency: f32,
  pub u_sss_width: f32,
  pub u_sss_bias: f32,
  pub u_sss_gain: f32,
  pub u_sss_strength: f32,
  // pub u_sss_position: Vec3,
}

unsafe impl bytemuck::Zeroable for ForwardModelUBO {}
unsafe impl bytemuck::Pod for ForwardModelUBO {}

fn flag_bits(cond: bool, bit: i32) -> i32 {
  if cond {
    bit
  } else {
    0
  }
}

impl ForwardModelUBO {
  pub fn new(entity: &WorldEntity, camera: &Camera) -> ForwardModelUBO {
    let material = &entity.material;
    let mut material_flags: i32 = 0;
    material_flags |= flag_bits(material.is_metallic, FLAG_IS_METALIC);
    material_flags |= flag_bits(material.specular_tex.is_some(), FLAG_USE_SPECULAR_TEXTURE);
    material_flags |= flag_bits(
      material.hair_shadow_tex.is_some(),
      FLAG_USE_HAIR_SHADOW_TEXTURE,
    );

    ForwardModelUBO {
      u_model_matrix: entity.model_matrix,
      u_model_view_projection_matrix: camera.model_view_projection_matrix(entity.model_matrix),
      u_specular: material.specular,
      u_specular_mul: material.specular_mul,
      u_material_flags: material_flags,
      u_sss_transluency: material.sss_transluency,
      u_sss_width: material.sss_width,
      u_sss_bias: material.sss_bias,
      u_sss_gain: material.sss_gain,
      u_sss_strength: material.sss_strength,
    }
  }
}
