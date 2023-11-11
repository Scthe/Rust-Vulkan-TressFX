mod tfx_forward_pass;
mod tfx_ppll_build_pass;
mod tfx_ppll_resolve_pass;

use crate::vk_utils::VkTexture;

pub use self::tfx_forward_pass::*;
pub use self::tfx_ppll_build_pass::*;
pub use self::tfx_ppll_resolve_pass::*;

use super::PassExecContext;

pub fn execute_tfx_ppll(
  tfx_ppll_build_pass: &TfxPpllBuildPass,
  tfx_ppll_resolve_pass: &TfxPpllResolvePass,
  pass_ctx: &PassExecContext,
  fbo_build: &mut TfxPpllBuildPassFramebuffer,
  fbo_resolve: &mut TfxPpllResolvePassFramebuffer,
  depth_stencil_tex: &mut VkTexture,
  forward_color_tex: &mut VkTexture,
) {
  pass_ctx.debug_start_pass("tfx_ppll_build_pass");
  tfx_ppll_build_pass.execute(&pass_ctx, fbo_build, depth_stencil_tex);

  pass_ctx.debug_start_pass("tfx_ppll_resolve_pass");
  tfx_ppll_resolve_pass.execute(
    &pass_ctx,
    fbo_resolve,
    depth_stencil_tex,
    forward_color_tex,
    &mut fbo_build.head_pointers_image,
    &mut fbo_build.ppll_data,
  );
}
