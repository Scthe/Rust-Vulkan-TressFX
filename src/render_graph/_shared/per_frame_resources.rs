use ash;
use ash::vk;

use crate::render_graph::blur_pass::BlurFramebuffer;
use crate::render_graph::forward_pass::ForwardPassFramebuffer;
use crate::render_graph::linear_depth_pass::LinearDepthPassFramebuffer;
use crate::render_graph::shadow_map_pass::ShadowMapPassFramebuffer;
use crate::render_graph::ssao_pass::SSAOPassFramebuffer;
use crate::render_graph::sss_blur_pass::SSSBlurFramebuffer;
use crate::render_graph::tfx_render::{TfxPpllBuildPassFramebuffer, TfxPpllResolvePassFramebuffer};
use crate::render_graph::tonemapping_pass::TonemappingPassFramebuffer;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::{VkBuffer, VkMemoryResource, VkTexture};

/// One instance per frame-in-flight.
pub struct PerFrameResources {
  // global ubos
  /// Refreshed once every frame. Contains e.g. all config settings, camera data
  pub config_uniform_buffer: VkBuffer,

  // framebuffers
  pub shadow_map_pass: ShadowMapPassFramebuffer,
  pub sss_depth_pass: ShadowMapPassFramebuffer,
  pub sss_blur_fbo0: SSSBlurFramebuffer,
  pub sss_blur_fbo1: SSSBlurFramebuffer,
  pub ssao_blur_fbo0: BlurFramebuffer,
  pub ssao_blur_fbo1: BlurFramebuffer,
  pub forward_pass: ForwardPassFramebuffer,
  pub tfx_ppll_build_pass: TfxPpllBuildPassFramebuffer,
  pub tfx_ppll_resolve_pass: TfxPpllResolvePassFramebuffer,
  pub tfx_depth_only_pass: vk::Framebuffer,
  pub linear_depth_pass: LinearDepthPassFramebuffer,
  pub ssao_pass: SSAOPassFramebuffer,
  pub tonemapping_pass: TonemappingPassFramebuffer,
  pub present_pass: vk::Framebuffer,

  // misc
  /// SSS - first result attachment in ping-pong
  pub sss_ping_result_tex: VkTexture,
  // SSAO - first result attachment in ping-pong
  pub ssao_ping_result_tex: VkTexture,
}

impl PerFrameResources {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    // buffers
    self.config_uniform_buffer.unmap_memory(allocator);
    self.config_uniform_buffer.delete(allocator);

    // passes framebuffers
    self.shadow_map_pass.destroy(vk_app);
    self.sss_depth_pass.destroy(vk_app);
    self.sss_blur_fbo0.destroy(vk_app);
    self.sss_blur_fbo1.destroy(vk_app);
    self.ssao_blur_fbo0.destroy(vk_app);
    self.ssao_blur_fbo1.destroy(vk_app);
    self.forward_pass.destroy(vk_app);
    device.destroy_framebuffer(self.tfx_depth_only_pass, None);
    self.tfx_ppll_build_pass.destroy(vk_app);
    self.tfx_ppll_resolve_pass.destroy(vk_app);
    self.linear_depth_pass.destroy(vk_app);
    self.ssao_pass.destroy(vk_app);
    self.tonemapping_pass.destroy(vk_app);
    device.destroy_framebuffer(self.present_pass, None);

    // misc
    self.sss_ping_result_tex.delete(device, allocator);
    self.ssao_ping_result_tex.delete(device, allocator);
  }
}
