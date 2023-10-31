use ash;
use ash::vk;

use crate::render_graph::forward_pass::ForwardPassFramebuffer;
use crate::render_graph::linear_depth_pass::LinearDepthPassFramebuffer;
use crate::render_graph::shadow_map_pass::ShadowMapPassFramebuffer;
use crate::render_graph::ssao_pass::SSAOPassFramebuffer;
use crate::render_graph::tonemapping_pass::TonemappingPassFramebuffer;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::{VkBuffer, VkMemoryResource};

/// One instance per frame-in-flight.
pub struct PerFrameResources {
  // global ubos
  /// Refreshed once every frame. Contains e.g. all config settings, camera data
  pub config_uniform_buffer: VkBuffer,

  // framebuffers
  pub shadow_map_pass: ShadowMapPassFramebuffer,
  pub sss_depth_pass: ShadowMapPassFramebuffer,
  pub forward_pass: ForwardPassFramebuffer,
  pub linear_depth_pass: LinearDepthPassFramebuffer,
  pub ssao_pass: SSAOPassFramebuffer,
  pub tonemapping_pass: TonemappingPassFramebuffer,
  pub present_pass: vk::Framebuffer,
}

impl PerFrameResources {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    // buffers
    self.config_uniform_buffer.unmap_memory(allocator);
    self.config_uniform_buffer.delete(allocator);

    // passes
    self.shadow_map_pass.destroy(vk_app);
    self.sss_depth_pass.destroy(vk_app);
    self.forward_pass.destroy(vk_app);
    self.linear_depth_pass.destroy(vk_app);
    self.ssao_pass.destroy(vk_app);
    self.tonemapping_pass.destroy(vk_app);
    device.destroy_framebuffer(self.present_pass, None);
  }
}
