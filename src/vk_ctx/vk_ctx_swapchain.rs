use ash;
use ash::extensions::khr::Swapchain;
use ash::vk;

pub struct VkCtxSwapchain {
  pub swapchain_loader: Swapchain,
  pub swapchain: vk::SwapchainKHR,
  pub size: vk::Extent2D,
  pub surface_format: vk::SurfaceFormatKHR,
}

impl VkCtxSwapchain {
  /// Will also destroy images. From validation layers:
  /// VK_OBJECT_TYPE_IMAGE; is a presentable image and it is controlled by the implementation and is destroyed with vkDestroySwapchainKHR.
  pub unsafe fn destroy(&self) {
    self
      .swapchain_loader
      .destroy_swapchain(self.swapchain, None);
  }
}
