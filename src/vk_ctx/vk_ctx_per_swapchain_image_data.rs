use ash;
use ash::vk;

/** Data per each frame-in-flight */
pub struct VkCtxPerSwapchainImageData {
  pub command_buffer: vk::CommandBuffer,

  // synchronize
  pub present_complete_semaphore: vk::Semaphore,
  pub rendering_complete_semaphore: vk::Semaphore,
  pub draw_command_fence: vk::Fence,
}
