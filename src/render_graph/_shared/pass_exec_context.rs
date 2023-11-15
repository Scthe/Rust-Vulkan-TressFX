use std::cell::RefCell;

use ash::vk;
use log::info;

use crate::{
  app_timer::AppTimer,
  config::Config,
  gpu_profiler::{GpuProfiler, ScopeId},
  scene::World,
  vk_ctx::VkCtx,
  vk_utils::{
    cmd_begin_render_pass_for_framebuffer, debug::add_render_pass_debug_label, ResouceBinder,
    VkBuffer,
  },
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
  pub timer: &'a AppTimer,
  /// Use `RefCell` to allow both mutable and const borrow regardles if `self` is mutable.
  pub profiler: RefCell<&'a mut GpuProfiler>,
}

impl PassExecContext<'_> {
  pub fn create_resouce_binder(&self, pipeline_layout: vk::PipelineLayout) -> ResouceBinder {
    ResouceBinder {
      push_descriptor: &self.vk_app.push_descriptor,
      command_buffer: self.command_buffer,
      pipeline_layout,
    }
  }

  pub unsafe fn cmd_start_render_pass(
    &self,
    name: &str,
    render_pass: &vk::RenderPass,
    framebuffer: &vk::Framebuffer,
    viewport_size: &vk::Extent2D,
    clear_values: &[vk::ClearValue],
  ) -> ScopeId {
    if self.config.only_first_frame {
      info!("Start {}", name);
    }
    add_render_pass_debug_label(&self.vk_app.debug_utils_loader, self.command_buffer, name);

    let device = self.vk_app.vk_device();
    cmd_begin_render_pass_for_framebuffer(
      &device,
      &self.command_buffer,
      &render_pass,
      &framebuffer,
      &viewport_size,
      clear_values,
    );

    self
      .profiler
      .borrow_mut()
      .begin_scope(device, self.command_buffer, name)
  }

  pub unsafe fn cmd_end_render_pass(&self, scope_id: ScopeId) {
    let device = self.vk_app.vk_device();
    device.cmd_end_render_pass(self.command_buffer);

    self
      .vk_app
      .debug_utils_loader
      .cmd_end_debug_utils_label(self.command_buffer);

    self
      .profiler
      .borrow()
      .end_scope(device, self.command_buffer, scope_id);
  }
}
