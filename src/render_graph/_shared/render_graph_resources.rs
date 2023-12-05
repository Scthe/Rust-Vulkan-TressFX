use ash;
use ash::vk;

use crate::config::Config;
use crate::render_graph::blur_pass::BlurFramebuffer;
use crate::render_graph::forward_pass::{ForwardPass, ForwardPassFramebuffer};
use crate::render_graph::linear_depth_pass::LinearDepthPassFramebuffer;
use crate::render_graph::shadow_map_pass::{ShadowMapPass, ShadowMapPassFramebuffer};
use crate::render_graph::ssao_pass::{SSAOPass, SSAOPassFramebuffer};
use crate::render_graph::sss_blur_pass::{SSSBlurFramebuffer, SSSBlurPass};
use crate::render_graph::sss_depth_pass::SSSDepthPassFramebuffer;
use crate::render_graph::tfx_render::{TfxPpllBuildPassFramebuffer, TfxPpllResolvePassFramebuffer};
use crate::render_graph::tonemapping_pass::TonemappingPassFramebuffer;
use crate::render_graph::RenderGraph;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::VkTexture;

pub struct RenderGraphResources {
  // framebuffers
  pub shadow_map_pass: ShadowMapPassFramebuffer,
  pub sss_depth_pass: SSSDepthPassFramebuffer,
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

  // misc
  /// SSS - first result attachment in ping-pong
  pub sss_ping_result_tex: VkTexture,
  /// SSAO - first result attachment in ping-pong
  pub ssao_ping_result_tex: VkTexture,
}

impl RenderGraphResources {
  pub fn new(vk_app: &VkCtx, config: &Config, rg: &RenderGraph) -> Self {
    let window_size = &vk_app.window_size();
    let ssao_result_size = config.get_ssao_viewport_size();

    // textures
    let sss_ping_result_tex = ForwardPass::create_diffuse_attachment_tex::<SSSBlurPass>(
      vk_app,
      "sss_blur_tmp",
      window_size,
    );
    let ssao_ping_result_tex = SSAOPass::create_result_texture(vk_app, &ssao_result_size, true);

    // fbos
    // shadow + shadow-like SSS
    let shadow_map_pass = rg
      .shadow_map_pass
      .create_framebuffer::<ShadowMapPass>(vk_app, config.shadows.shadowmap_size);
    let sss_depth_pass = rg.sss_depth_pass.create_framebuffer(
      vk_app,
      &rg.shadow_map_pass,
      config.sss_forward_scatter.depthmap_size,
    );
    // forward
    let forward_pass = rg.forward_pass.create_framebuffer(vk_app, window_size);
    // tfx
    let tfx_ppll_build_pass = rg
      .tfx_ppll_build_pass
      .create_framebuffer(vk_app, &forward_pass.depth_stencil_tex);
    let tfx_ppll_resolve_pass = rg.tfx_ppll_resolve_pass.create_framebuffer(
      vk_app,
      &forward_pass.depth_stencil_tex,
      &forward_pass.diffuse_tex,
      &forward_pass.normals_tex,
    );

    let tfx_depth_only_pass = rg
      .tfx_depth_only_pass
      .create_framebuffer(vk_app, &forward_pass.depth_stencil_tex);
    // sss blur
    let sss_blur_fbo0 = rg.sss_blur_pass.create_framebuffer(
      vk_app,
      &forward_pass.depth_stencil_tex,
      &sss_ping_result_tex,
    );
    let sss_blur_fbo1 = rg.sss_blur_pass.create_framebuffer(
      vk_app,
      &forward_pass.depth_stencil_tex,
      &forward_pass.diffuse_tex,
    );
    // linear depth
    let linear_depth_pass = rg.linear_depth_pass.create_framebuffer(vk_app, window_size);
    // ssao
    let ssao_pass = rg.ssao_pass.create_framebuffer(vk_app, &ssao_result_size);
    let ssao_blur_fbo0 = rg
      .ssao_blur_pass
      .create_framebuffer(vk_app, &ssao_ping_result_tex);
    let ssao_blur_fbo1 = rg
      .ssao_blur_pass
      .create_framebuffer(vk_app, &ssao_pass.ssao_tex);
    // tonemap
    let tonemapping_pass = rg.tonemapping_pass.create_framebuffer(vk_app, window_size);

    Self {
      // fbos
      shadow_map_pass,
      sss_depth_pass,
      sss_blur_fbo0,
      sss_blur_fbo1,
      forward_pass,
      tfx_ppll_build_pass,
      tfx_ppll_resolve_pass,
      tfx_depth_only_pass,
      linear_depth_pass,
      ssao_pass,
      ssao_blur_fbo0,
      ssao_blur_fbo1,
      tonemapping_pass,
      // textures
      sss_ping_result_tex,
      ssao_ping_result_tex,
    }
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

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

    // misc
    self.sss_ping_result_tex.delete(device, allocator);
    self.ssao_ping_result_tex.delete(device, allocator);
  }
}
