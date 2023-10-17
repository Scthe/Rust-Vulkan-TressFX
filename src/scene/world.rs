use crate::vk_utils::{VkBuffer, VkTexture};

use super::Camera;

pub struct WorldEntity {
  pub name: String,

  // mesh:
  pub vertex_buffer: VkBuffer,
  pub index_buffer: VkBuffer,
  pub vertex_count: u32,
  pub tex_diffuse: VkTexture,
  // TODO material
  // pub uniforms_buffer: VkBuffer, // material+tfx+..., bind as descriptor set
  // TODO tfx? Or just precalc hardcoded model matrix. We have static data here..
}

impl WorldEntity {
  pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    self.vertex_buffer.delete(allocator);
    self.index_buffer.delete(allocator);
    self.tex_diffuse.delete(device, allocator);
  }
}

pub struct World {
  pub camera: Camera,
  pub entities: Vec<WorldEntity>,
}

impl World {
  pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    for entity in &mut self.entities {
      entity.destroy(device, allocator);
    }
  }
}
