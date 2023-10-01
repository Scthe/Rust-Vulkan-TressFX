use bytemuck;
use glam::Vec4;

use crate::vk_utils::VkBuffer;

#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct TriangleVertex {
  pos: Vec4, // TODO Vec2, Vec3 are enough
  color: Vec4,
}

impl TriangleVertex {
  pub fn new(pos: (f32, f32), col: (f32, f32, f32)) -> TriangleVertex {
    TriangleVertex {
      pos: Vec4::new(pos.0, pos.1, 0.0f32, 1.0f32),
      color: Vec4::new(col.0, col.1, col.2, 1.0f32),
    }
  }
}

unsafe impl bytemuck::Zeroable for TriangleVertex {}
unsafe impl bytemuck::Pod for TriangleVertex {}

/// TODO camera, objects{meshes, materials, tfx}[]
pub struct World {
  pub triangle_vertex_buffer: VkBuffer,
}

impl World {
  pub fn destroy(&self, allocator: &vk_mem::Allocator) -> () {
    self.triangle_vertex_buffer.delete(allocator);
  }
}
