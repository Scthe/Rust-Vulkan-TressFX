use ash;
use ash::vk;

pub struct VkCtxCommandBuffers {
  pub pool: vk::CommandPool,
  // Special command buffer used for resource init
  pub setup_cb: vk::CommandBuffer,
  // one per each swapchain image:
  pub cmd_buffers: Vec<vk::CommandBuffer>,
}

impl VkCtxCommandBuffers {
  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_command_pool(self.pool, None);
  }
}
