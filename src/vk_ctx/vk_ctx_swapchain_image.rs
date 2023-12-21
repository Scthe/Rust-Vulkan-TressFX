use ash;
use ash::vk;

use crate::vk_utils::{
  create_image_barrier, create_swapchain_image_view, VkStorageResourceBarrier,
};

/// https://www.khronos.org/assets/uploads/developers/library/2016-vulkan-devday-uk/7-Keeping-your-GPU-fed.pdf
pub struct VkCtxSwapchainImage {
  pub index: usize,
  /// auto destroyed with swapchain
  pub image: vk::Image,
  pub image_view: vk::ImageView,
}

impl VkCtxSwapchainImage {
  pub fn new(
    device: &ash::Device,
    index: usize,
    image: vk::Image,
    image_format: vk::Format,
  ) -> Self {
    Self {
      index,
      image,
      image_view: create_swapchain_image_view(device, image, image_format),
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_image_view(self.image_view, None);
  }

  /// Do not call this during frame loop. Use only during init.
  /// Runtime transition would require previous layout/access,
  /// which complicates things.
  pub fn create_barrier_transition_to_present_layout(&self) -> vk::ImageMemoryBarrier2 {
    let mut barrier = VkStorageResourceBarrier::empty();
    barrier.previous_op.0 = vk::PipelineStageFlags2::TOP_OF_PIPE;
    barrier.next_op.0 = vk::PipelineStageFlags2::FRAGMENT_SHADER;

    create_image_barrier(
      self.image,
      vk::ImageAspectFlags::COLOR,
      vk::ImageLayout::UNDEFINED,
      vk::ImageLayout::PRESENT_SRC_KHR,
      barrier,
    )
  }
}
