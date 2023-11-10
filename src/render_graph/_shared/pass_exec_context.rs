use ash::vk;
use log::info;

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
  pub config: &'a mut Config, // `mut` cause UI
  pub scene: &'a mut World,   // `mut` cause UI
  pub command_buffer: vk::CommandBuffer,
  pub size: vk::Extent2D,
  pub config_buffer: &'a VkBuffer,
  pub window: &'a winit::window::Window,
}

impl PassExecContext<'_> {
  pub fn create_resouce_binder(&self, pipeline_layout: vk::PipelineLayout) -> ResouceBinder {
    ResouceBinder {
      push_descriptor: &self.vk_app.push_descriptor,
      command_buffer: self.command_buffer,
      pipeline_layout,
    }
  }

  pub fn debug_start_pass(&self, name: &str) {
    if self.config.only_first_frame {
      info!("Start {}", name);
    }
  }
}
