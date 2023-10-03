use crate::vk_utils::VkBuffer;

/// TODO camera, objects{meshes, materials, tfx}[]
pub struct World {
  pub triangle_vertex_buffer: VkBuffer,
}

impl World {
  pub fn destroy(&self, allocator: &vk_mem::Allocator) -> () {
    self.triangle_vertex_buffer.delete(allocator);
  }
}
