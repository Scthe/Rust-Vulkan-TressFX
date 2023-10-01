use ash;
use ash::version::DeviceV1_0;
use ash::vk;

pub struct VkCtxCommandBuffers {
  pub pool: vk::CommandPool,
  // one per each swapchain image:
  pub cmd_buffers: Vec<vk::CommandBuffer>,
}

impl VkCtxCommandBuffers {
  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_command_pool(self.pool, None);
  }
}
