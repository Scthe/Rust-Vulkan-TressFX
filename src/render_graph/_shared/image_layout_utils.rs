use ash;
use ash::vk;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::create_image_barrier;
use crate::vk_utils::VkStorageResourceBarrier;
use crate::vk_utils::WithSetupCmdBuffer;

pub fn transition_window_framebuffers_for_present_khr(vk_app: &VkCtx) {
  vk_app.with_setup_cb(|device, cmd_buf| {
    let barriers = vk_app
      .swapchain_images
      .iter()
      .map(|swapchain_image| {
        let mut barrier = VkStorageResourceBarrier::empty();
        barrier.previous_op.0 = vk::PipelineStageFlags2::TOP_OF_PIPE;
        barrier.next_op.0 = vk::PipelineStageFlags2::FRAGMENT_SHADER;

        create_image_barrier(
          swapchain_image.image,
          vk::ImageAspectFlags::COLOR,
          vk::ImageLayout::UNDEFINED,
          vk::ImageLayout::PRESENT_SRC_KHR,
          barrier,
        )
      })
      .collect::<Vec<_>>();

    unsafe {
      let dep = vk::DependencyInfo::builder().image_memory_barriers(&barriers);
      device.cmd_pipeline_barrier2(cmd_buf, &dep);
    };
  });
}
