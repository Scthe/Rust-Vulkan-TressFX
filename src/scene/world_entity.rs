use std::mem::size_of;

use ash;
use ash::vk;
use glam::Mat4;

use crate::{
  config::Config,
  render_graph::ForwardModelUBO,
  vk_ctx::VkCtx,
  vk_utils::{VkBuffer, VkMemoryResource},
};

use super::{BoundingBox, Camera, Material};

pub struct WorldEntity {
  pub name: String,
  pub model_matrix: Mat4,
  /// more for debug and scale comparison than culling
  pub aabb: BoundingBox,

  // mesh
  pub vertex_buffer: VkBuffer,
  pub index_buffer: VkBuffer,
  pub vertex_count: u32,
  /// material+textures
  pub material: Material,
  /// Model data uploaded to GPU. Refreshed every frame (cause mvp matrices, changes from ui etc.)
  pub model_ubo: Vec<VkBuffer>,
}

impl WorldEntity {
  pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    self.vertex_buffer.delete(allocator);
    self.index_buffer.delete(allocator);
    self.material.destroy(device, allocator);
    self.model_ubo.iter_mut().for_each(|buffer| {
      buffer.unmap_memory(allocator);
      buffer.delete(allocator);
    })
  }

  pub fn get_ubo_buffer(&self, frame_id: usize) -> &VkBuffer {
    &self.model_ubo[frame_id]
  }

  pub fn update_ubo_data(&self, frame_id: usize, config: &Config, camera: &Camera) {
    let data = ForwardModelUBO::new(config, self, camera);
    let data_bytes = bytemuck::bytes_of(&data);
    let buffer = self.get_ubo_buffer(frame_id);
    buffer.write_to_mapped(data_bytes);
  }

  pub unsafe fn cmd_bind_mesh_buffers(
    &self,
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
  ) {
    device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.buffer], &[0]);
    device.cmd_bind_index_buffer(
      command_buffer,
      self.index_buffer.buffer,
      0,
      vk::IndexType::UINT32,
    );
  }

  pub unsafe fn cmd_draw_mesh(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
    device.cmd_draw_indexed(command_buffer, self.vertex_count, 1, 0, 0, 0);
  }
}

fn allocate_model_ubo(vk_ctx: &VkCtx, name: &str, frame_idx: usize) -> VkBuffer {
  let allocator = &vk_ctx.allocator;
  let size = size_of::<ForwardModelUBO>() as _;

  let mut buffer = VkBuffer::empty(
    format!("{}.model_ubo#{}", name, frame_idx),
    size,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
    allocator,
    vk_ctx.device.queue_family_index,
    true,
  );
  buffer.map_memory(allocator); // always mapped
  buffer
}

pub fn allocate_model_ubo_vec(vk_ctx: &VkCtx, name: &str) -> Vec<VkBuffer> {
  let in_flight_frames = vk_ctx.frames_in_flight();
  (0..in_flight_frames)
    .map(|i| allocate_model_ubo(vk_ctx, &name, i))
    .collect::<Vec<_>>()
}
