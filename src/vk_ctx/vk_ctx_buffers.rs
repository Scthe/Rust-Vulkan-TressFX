use crate::vk_utils::VkBuffer;

pub struct VkAppBuffers {
  pub triangle_vertex_buffer: VkBuffer,
}

impl VkAppBuffers {
  pub unsafe fn destroy(&self, allocator: &vk_mem::Allocator) {
    self.triangle_vertex_buffer.delete(allocator);
  }
}
