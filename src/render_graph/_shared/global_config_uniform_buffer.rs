use bytemuck;
use glam::Mat4;

#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct GlobalConfigUniformBuffer {
  // view projection matrix for current camera
  pub u_vp: Mat4,
}

unsafe impl bytemuck::Zeroable for GlobalConfigUniformBuffer {}
unsafe impl bytemuck::Pod for GlobalConfigUniformBuffer {}

impl GlobalConfigUniformBuffer {
  // must be same as in shader!
  pub const BINDING_INDEX: u32 = 0;
  // TODO this is tmp, texture should not be part of this shader
  pub const TMP_TEXTURE_BINDING_INDEX: u32 = 1;
}
