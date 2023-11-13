use bytemuck;
use glam::{vec4, Mat4, Vec4};

use crate::{
  config::Config,
  scene::TfxObject,
  utils::{into_vec4, mint3_into_vec4},
};

#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct TfxParamsUBO {
  pub u_model_matrix: Mat4,
  pub u_general_settings: Vec4, // [opacity, uint u_numVerticesPerStrand, u_tfx_ao_strength, u_tfx_ao_exp]
  // geometry
  pub u_geometry: Vec4, // [u_thin_tip, u_fiber_radius, u_follow_hair_spread_root, u_follow_hair_spread_tip]
  pub u_center_of_gravity: Vec4, // [cog.xyz, -]
  // material
  pub u_albedo: Vec4,    // [u_albedo.rgb, -]
  pub u_specular1: Vec4, // [u_specularColor1.rgb, u_specular_power1]
  pub u_specular2: Vec4, // [u_specularColor1.rgb, u_specular_power2]
  pub u_material: Vec4, // [u_primaryShift, u_secondaryShift, u_specularStrength1, u_specularStrength2]
}

unsafe impl bytemuck::Zeroable for TfxParamsUBO {}
unsafe impl bytemuck::Pod for TfxParamsUBO {}

impl TfxParamsUBO {
  pub fn new(_config: &Config, tfx: &TfxObject) -> Self {
    let mat = &tfx.material;

    Self {
      u_model_matrix: tfx.model_matrix,
      u_general_settings: vec4(
        mat.opacity,
        tfx.num_vertices_per_strand as f32,
        mat.ao_strength,
        mat.ao_exp,
      ),
      u_geometry: vec4(
        1.0 - tfx.thin_tip,
        tfx.fiber_radius,
        tfx.follow_hair_spread_root,
        tfx.follow_hair_spread_tip,
      ),
      u_center_of_gravity: into_vec4(tfx.center_of_gravity, 0.0),
      u_albedo: mint3_into_vec4(mat.albedo, 0.0),
      u_specular1: mint3_into_vec4(mat.specular_color1, mat.specular_power1),
      u_specular2: mint3_into_vec4(mat.specular_color2, mat.specular_power2),
      u_material: vec4(
        mat.primary_shift,
        mat.secondary_shift,
        mat.specular_strength1,
        mat.specular_strength2,
      ),
    }
  }
}
