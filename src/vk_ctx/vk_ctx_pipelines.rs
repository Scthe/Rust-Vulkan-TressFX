use ash;
use ash::version::DeviceV1_0;
use ash::vk;

pub struct VkAppPipelines {
  pub pipeline_cache: vk::PipelineCache,
  // app specific:
  pub pipeline_triangle: vk::Pipeline,
  pub pipeline_triangle_layout: vk::PipelineLayout,
}

impl VkAppPipelines {
  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_pipeline_layout(self.pipeline_triangle_layout, None);
    device.destroy_pipeline(self.pipeline_triangle, None);
    device.destroy_pipeline_cache(self.pipeline_cache, None);
  }
}
