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

  /// TODO [MEDIUM] This fn assumes last access was COLOR_ATTACHMENT_WRITE,
  ///      but it might have been TRANSFER_WRITE as well.
  pub fn create_barrier_transition_to_present_layout(&self) -> vk::ImageMemoryBarrier2 {
    let mut barrier = VkStorageResourceBarrier::empty();
    barrier.previous_op.0 = vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT;
    barrier.previous_op.1 = vk::AccessFlags2::COLOR_ATTACHMENT_WRITE;
    // https://github.com/KhronosGroup/Vulkan-Samples/blob/d9a6b1069f8008e83a74ae6c08fc7b0235aa2830/framework/common/vk_common.cpp#L426
    barrier.next_op.0 = vk::PipelineStageFlags2::BOTTOM_OF_PIPE;
    // https://github.com/KhronosGroup/Vulkan-Samples/blob/d9a6b1069f8008e83a74ae6c08fc7b0235aa2830/framework/common/vk_common.cpp#L396
    barrier.next_op.1 = vk::AccessFlags2::NONE;

    create_image_barrier(
      self.image,
      vk::ImageAspectFlags::COLOR,
      vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
      vk::ImageLayout::PRESENT_SRC_KHR,
      barrier,
    )
  }
}
