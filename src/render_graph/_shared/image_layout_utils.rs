use ash;
use ash::vk;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::create_image_barrier;
use crate::vk_utils::WithSetupCmdBuffer;

pub fn transition_window_framebuffers_for_present_khr(vk_app: &VkCtx) {
  vk_app.with_setup_cb(|device, cmd_buf| {
    let barriers = vk_app
      .swapchain
      .images
      .iter()
      .map(|image| {
        create_image_barrier(
          *image,
          vk::ImageAspectFlags::COLOR,
          vk::ImageLayout::UNDEFINED,
          vk::ImageLayout::PRESENT_SRC_KHR,
          vk::AccessFlags::empty(),
          vk::AccessFlags::empty(),
        )
      })
      .collect::<Vec<_>>();

    unsafe {
      device.cmd_pipeline_barrier(
        cmd_buf,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &barriers,
      )
    };
  });
}
