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
