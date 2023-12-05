use ash;
use ash::vk;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::VkBuffer;

/// One instance per frame-in-flight.
pub struct FrameData {
  pub queue_submit_finished_fence: vk::Fence,
  pub command_buffer: vk::CommandBuffer,
  /// Refreshed once every frame. Contains e.g. all config settings, camera data
  pub config_uniform_buffer: VkBuffer,
}

impl FrameData {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    self.config_uniform_buffer.delete(allocator);
    device.destroy_fence(self.queue_submit_finished_fence, None);
  }
}
