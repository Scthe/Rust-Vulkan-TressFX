use ash;
use ash::version::DeviceV1_0;
use ash::vk;

pub struct VkAppRenderPasses {
  // app specific:
  pub render_pass_triangle: vk::RenderPass,
}

impl VkAppRenderPasses {
  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass_triangle, None);
  }
}
