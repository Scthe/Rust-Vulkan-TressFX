use crate::vk_utils::VkBuffer;

use super::Camera;

pub struct WorldEntity {
  pub name: String,

  // mesh:
  pub vertex_buffer: VkBuffer,
  pub index_buffer: VkBuffer,
  pub vertex_count: u32,
  // TODO material
  // pub uniforms_buffer: VkBuffer, // material+tfx+..., bind as descriptor set
  // TODO tfx? Or just precalc hardcoded model matrix. We have static data here..
}

pub struct World {
  pub camera: Camera,
  pub entities: Vec<WorldEntity>,
}

impl World {
  pub unsafe fn destroy(&mut self, allocator: &vma::Allocator) -> () {
    for entity in &mut self.entities {
      entity.vertex_buffer.delete(allocator);
      entity.index_buffer.delete(allocator);
    }
  }
}
