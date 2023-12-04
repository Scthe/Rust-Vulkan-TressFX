use ash;
use ash::vk;

use crate::vk_utils::create_semaphore;

/// https://www.khronos.org/assets/uploads/developers/library/2016-vulkan-devday-uk/7-Keeping-your-GPU-fed.pdf
pub struct VkCtxSwapchainImage {
  pub image: vk::Image,
  pub image_view: vk::ImageView,
  pub acquire_semaphore: vk::Semaphore,
  /// `release_semaphore`
  pub rendering_complete_semaphore: vk::Semaphore,
}

impl VkCtxSwapchainImage {
  pub fn new(device: &ash::Device, image: vk::Image, image_view: vk::ImageView) -> Self {
    Self {
      image,
      image_view,
      acquire_semaphore: create_semaphore(device),
      rendering_complete_semaphore: create_semaphore(device),
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_semaphore(self.acquire_semaphore, None);
    device.destroy_semaphore(self.rendering_complete_semaphore, None);
    device.destroy_image_view(self.image_view, None);
  }
}
