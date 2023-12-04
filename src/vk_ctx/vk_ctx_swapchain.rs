use ash;
use ash::extensions::khr::Swapchain;
use ash::vk;

pub struct VkCtxSwapchain {
  pub swapchain_loader: Swapchain,
  pub swapchain: vk::SwapchainKHR,
  pub size: vk::Extent2D,
  pub surface_format: vk::SurfaceFormatKHR,

  // All fields below will be capabilites.min_images + 1
  pub image_views: Vec<vk::ImageView>,
  pub images: Vec<vk::Image>,
}

impl VkCtxSwapchain {
  /// Will also destroy images. From validation layers:
  /// VK_OBJECT_TYPE_IMAGE; is a presentable image and it is controlled by the implementation and is destroyed with vkDestroySwapchainKHR.
  pub unsafe fn destroy(&self, device: &ash::Device) {
    for &image_view in &self.image_views {
      device.destroy_image_view(image_view, None);
    }

    self
      .swapchain_loader
      .destroy_swapchain(self.swapchain, None);
  }
}
