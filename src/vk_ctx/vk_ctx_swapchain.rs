use ash;
use ash::extensions::khr::Swapchain;
use ash::version::DeviceV1_0;
use ash::vk;

pub struct VkAppSwapchain {
  pub swapchain_loader: Swapchain,
  pub swapchain: vk::SwapchainKHR,
  pub size: vk::Extent2D,

  // All fields below will be capabilites.min_images + 1
  pub framebuffers: Vec<vk::Framebuffer>,
  pub image_views: Vec<vk::ImageView>,
  pub images: Vec<vk::Image>,
}

impl VkAppSwapchain {
  pub unsafe fn destroy(&self, device: &ash::Device) {
    for &framebuffer in &self.framebuffers {
      device.destroy_framebuffer(framebuffer, None);
    }

    for &image_view in &self.image_views {
      device.destroy_image_view(image_view, None);
    }

    // Will also destroy images. From validation layers:
    // VK_OBJECT_TYPE_IMAGE; is a presentable image and it is controlled by the implementation and is destroyed with vkDestroySwapchainKHR.
    self
      .swapchain_loader
      .destroy_swapchain(self.swapchain, None);
  }
}
