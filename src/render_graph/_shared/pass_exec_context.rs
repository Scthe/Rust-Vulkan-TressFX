use ash::vk;

use crate::{
  config::Config,
  scene::World,
  vk_ctx::VkCtx,
  vk_utils::{ResouceBinder, VkBuffer},
};

/// All the kitchen sink that we might want to use in the render pass.
/// Created so we do not have to provide it all one-by-one.
pub struct PassExecContext<'a> {
  /// Index of the swapchain image, range: [0, frames_in_flight)
  pub swapchain_image_idx: usize,
  pub vk_app: &'a VkCtx,
  pub config: &'a Config,
  pub scene: &'a World,
  pub command_buffer: vk::CommandBuffer,
  pub size: vk::Extent2D,
  pub config_buffer: &'a VkBuffer,
}

impl PassExecContext<'_> {
  pub fn create_resouce_binder(&self, pipeline_layout: vk::PipelineLayout) -> ResouceBinder {
    ResouceBinder {
      push_descriptor: &self.vk_app.push_descriptor,
      command_buffer: self.command_buffer,
      pipeline_layout,
    }
  }
}
