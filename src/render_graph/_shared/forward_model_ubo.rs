use bytemuck;
use glam::Mat4;

// All below must match shader-defined consts
pub const FLAG_IS_METALIC: i32 = 1;
pub const FLAG_USE_SPECULAR_TEXTURE: i32 = 2;
pub const FLAG_USE_HAIR_SHADOW_TEXTURE: i32 = 4;

// add mvp here? If we add ui, we have to reupload anyway cuz materials can change
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct ForwardModelUBO {
  pub u_model_matrix: Mat4,
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
