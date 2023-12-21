use ash;
use ash::vk;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::{create_command_buffer, create_fence, create_semaphore, VkBuffer};

/// One instance per frame in flight.
pub struct FrameData {
  pub command_buffer: vk::CommandBuffer,
  /// Refreshed once every frame. Contains e.g. all config settings, camera data
  pub config_uniform_buffer: VkBuffer,

  // SYNC
  pub queue_submit_finished_fence: vk::Fence,
  pub acquire_semaphore: vk::Semaphore,
  /// `release_semaphore`
  pub rendering_complete_semaphore: vk::Semaphore,
}

impl FrameData {
  pub fn new(vk_app: &VkCtx, config_uniform_buffer: VkBuffer) -> Self {
    let device = vk_app.vk_device();
    let command_buffer = create_command_buffer(device, vk_app.command_pool);

    Self {
      command_buffer,
      config_uniform_buffer,
      queue_submit_finished_fence: create_fence(device),
      acquire_semaphore: create_semaphore(device),
      rendering_complete_semaphore: create_semaphore(device),
    }
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    self.config_uniform_buffer.delete(allocator);
    device.destroy_fence(self.queue_submit_finished_fence, None);
    device.destroy_semaphore(self.acquire_semaphore, None);
    device.destroy_semaphore(self.rendering_complete_semaphore, None);
  }
}
