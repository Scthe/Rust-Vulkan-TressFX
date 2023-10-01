use ash;
use ash::version::DeviceV1_0;
use ash::vk;

pub struct VkCtxRenderPasses {
  // app specific:
  pub render_pass_triangle: vk::RenderPass,
}

impl VkCtxRenderPasses {
  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass_triangle, None);
  }
}
