use ash;
use ash::vk;

/** Data per each frame-in-flight */
pub struct VkCtxPerSwapchainImageData {
  /// Index of the swapchain image, range: [0, frames_in_flight)
  pub swapchain_image_idx: usize,
  pub command_buffer: vk::CommandBuffer,

  // synchronize
  pub swapchain_image_acquired_semaphore: vk::Semaphore,
  pub queue_submit_finished_semaphore: vk::Semaphore,
  pub queue_submit_finished_fence: vk::Fence,
}
