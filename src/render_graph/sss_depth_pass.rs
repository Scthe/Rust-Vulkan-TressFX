use glam::Mat4;
use log::info;

use crate::config::ShadowSourceCfg;
use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;

use super::shadow_map_pass::{ShadowMapPass, ShadowMapPassFramebuffer};
use super::PassExecContext;

pub type SSSDepthPassFramebuffer = ShadowMapPassFramebuffer;

pub struct SSSDepthPass {}

/// Forward scatter: generate depth from point of view of SSS light.
///
/// In forward pass, we will calculate distance between it and
/// camera's depth to detect thin object like ears (when SSS light
/// behind and the camera en face). Light shines through thin objects.
impl SSSDepthPass {
  pub fn new() -> Self {
    info!("Creating {}", get_simple_type_name::<Self>());
    Self {}
  }

  pub unsafe fn destroy(&self) {}

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    frame_id: usize,
    shadow_pass: &ShadowMapPass,
    size_px: u32,
  ) -> SSSDepthPassFramebuffer {
    shadow_pass.create_framebuffer::<Self>(vk_app, frame_id, size_px)
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut SSSDepthPassFramebuffer,
    shadow_pass: &ShadowMapPass,
    source: &ShadowSourceCfg,
  ) -> () {
    shadow_pass.execute::<Self>(exec_ctx, framebuffer, source, false);
  }

  pub fn get_sss_forward_mvp(source: &ShadowSourceCfg, model_matrix: Mat4) -> Mat4 {
    ShadowMapPass::get_light_shadow_mvp(source, model_matrix)
  }
}
