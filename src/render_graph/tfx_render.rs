use ash;
use ash::vk;

mod tfx_depth_only_pass;
mod tfx_forward_pass;
mod tfx_ppll_build_pass;
mod tfx_ppll_resolve_pass;

use crate::vk_utils::VkTexture;

pub use self::tfx_depth_only_pass::*;
pub use self::tfx_forward_pass::*;
pub use self::tfx_ppll_build_pass::*;
pub use self::tfx_ppll_resolve_pass::*;

use super::PassExecContext;

/// Normally you would render all TressFX objects in PPLL build phase,
/// then run PPLL resolve only once. In resolve step, the material data
/// can be retrieved by:
/// - passing it directly in `PerPixelListEntryData` struct, or
/// - using global `TfxMaterialData[]` buffer and having
///     `PerPixelListEntryData.materialId` to [index into it](https://github.com/GPUOpen-Effects/TressFX/blob/ba0bdacdfb964e38522fda812bf23169bc5fa603/src/Shaders/TressFXPPLL.hlsl#L224).
///
/// For our simple demo app (with only 1 TressFX asset), we can just
/// run both build and resolve steps per each `TfxObject`.
/// Material data is then provided as a uniform. In this way
/// we also do clears for transient PPLL build pass (head pointers,
/// atomic counter) once per object - ineffective. Yet it really
/// does not matter for a single object.
pub fn execute_tfx_ppll(
  tfx_ppll_build_pass: &TfxPpllBuildPass,
  tfx_ppll_resolve_pass: &TfxPpllResolvePass,
  tfx_depth_only_pass: &TfxDepthOnlyPass,
  pass_ctx: &PassExecContext,
  fbo_build: &mut TfxPpllBuildPassFramebuffer,
  fbo_resolve: &mut TfxPpllResolvePassFramebuffer,
  fbo_depth_only: vk::Framebuffer,
  depth_stencil_tex: &mut VkTexture,
  forward_color_tex: &mut VkTexture,
  ao_texture: &mut VkTexture,
  shadow_map_texture: &mut VkTexture,
) {
  let scene = &*pass_ctx.scene;
  for entity in &scene.tressfx_objects {
    pass_ctx.debug_start_pass(&format!("tfx_ppll_build_pass.{}", entity.name));
    tfx_ppll_build_pass.execute(&pass_ctx, fbo_build, depth_stencil_tex, entity);

    pass_ctx.debug_start_pass(&format!("tfx_ppll_resolve_pass.{}", entity.name));
    tfx_ppll_resolve_pass.execute(
      &pass_ctx,
      fbo_resolve,
      depth_stencil_tex,
      forward_color_tex,
      &mut fbo_build.head_pointers_image,
      &mut fbo_build.ppll_data,
      ao_texture,
      shadow_map_texture,
      entity,
    );

    // Both build and resolve passess use early depth stencil tests
    // - build pass - requires to test depth before fragment shader starts writing to SSBO.
    //     Depth write disabled as it would self-occlude and we want to process all fragments,
    //     regardless of their depth.
    // - resolve pass - discard pixels that do not pass stencil test (huge optimization)
    // This means that depth buffer is never written to. Fix this mistake here.
    pass_ctx.debug_start_pass(&format!("tfx_depth_only_pass.{}", entity.name));
    tfx_depth_only_pass.execute(&pass_ctx, fbo_depth_only, depth_stencil_tex, entity);
  }
}
