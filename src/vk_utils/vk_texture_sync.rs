use log::trace;

use ash;
use ash::vk;

use crate::vk_utils::create_image_barrier;

use super::VkMemoryResource;
use super::VkTexture;

// https://github.com/Tobski/simple_vulkan_synchronization/blob/main/thsvs_simpler_vulkan_synchronization.h
// https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples-(Legacy-synchronization-APIs)

const DEBUG_LAYOUT_TRANSITIONS: bool = false;

impl VkTexture {
  /// The `srcStageMask` marks the stages to wait for in previous commands
  /// before allowing the stages given in `dstStageMask` to execute
  /// in subsequent commands.
  ///
  /// ## Docs
  /// * https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples-(Legacy-synchronization-APIs)
  /// * https://www.khronos.org/blog/understanding-vulkan-synchronization
  /// * https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAccessFlagBits.html
  /// * https://vulkan-tutorial.com/Texture_mapping/Images#page_Transition-barrier-masks
  ///
  /// ## Params:
  /// * `new_layout` - next layout to set to e.g. `COLOR_ATTACHMENT_OPTIMAL`
  ///     or `SHADER_READ_ONLY_OPTIMAL`
  /// * `src_access_mask` - previous op e.g. `COLOR_ATTACHMENT_WRITE`
  /// * `dst_access_mask` - op we will do e.g. `COLOR_ATTACHMENT_READ`
  ///
  pub fn barrier_prepare_for_layout_transition(
    &mut self,
    new_layout: vk::ImageLayout,
    src_access_mask: vk::AccessFlags,
    dst_access_mask: vk::AccessFlags,
  ) -> vk::ImageMemoryBarrier {
    self.trace_log_layout_transition("", new_layout);

    // Best practices, will require VK_PIPELINE_STAGE_HOST_BIT. Triggered only on the first use.
    // Please set the texture to proper layout after you create it!
    let src_access_mask2 = if self.layout == vk::ImageLayout::PREINITIALIZED {
      vk::AccessFlags::HOST_WRITE
    } else {
      src_access_mask
    };

    let barrier = create_image_barrier(
      self.image,
      self.aspect_flags,
      self.layout,
      new_layout,
      src_access_mask2,
      dst_access_mask,
    );

    self.layout = new_layout;
    barrier
  }

  /// TODO [???] return Option if layout already matches (e.g. read-after-read)?
  ///            Only for reads, as writes need barrier for write-after-write
  fn barrier_prepare_attachment_for_shader_read(&mut self) -> vk::ImageMemoryBarrier {
    if self.is_color() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE, // prev op
        vk::AccessFlags::SHADER_READ, //| vk::AccessFlags::INPUT_ATTACHMENT_READ (subpass only?), // our op
      )
    } else if self.is_depth_stencil() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, // prev op
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,  // our op
      )
    } else if self.is_depth() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, // prev op
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,  // our op
      )
    } else {
      panic!("Tried to transition texture {} for shader read, but it's neither color or depth-stencil texture.", self.get_name());
    }
  }

  fn barrier_prepare_attachment_for_write(&mut self) -> vk::ImageMemoryBarrier {
    if self.is_color() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::SHADER_READ,            // prev op
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE, // our op
      )
    } else if self.is_depth_stencil() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ, // prev op TODO what if this was actually write?
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, // our op
      )
    } else if self.is_depth() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ, // prev op
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ // our op
          | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
      )
    } else {
      panic!("Tried to transition texture {} for shader write, but it's neither color or depth-stencil texture.", self.get_name());
    }
  }

  pub(super) fn trace_log_layout_transition(&mut self, tag: &str, new_layout: vk::ImageLayout) {
    if DEBUG_LAYOUT_TRANSITIONS {
      trace!(
        "VkTexture::LayoutTransition {} '{}' ({:?} -> {:?})",
        tag,
        self.get_name(),
        self.layout,
        new_layout
      );
    }
  }

  /// Most common layout transition between passes are attachment `read->write` or `write->read` or `write->write`.
  ///
  /// Util to wrap the barrier code to make the attachments **READABLE IN FRAGMENT SHADER**
  /// (no depth/stencil test).
  pub unsafe fn cmd_transition_attachments_for_read_barrier(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    attachments: &mut [&mut VkTexture],
  ) {
    let mut prev_op_stage = vk::PipelineStageFlags::empty();
    let mut current_op_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;

    let barriers = attachments
      .iter_mut()
      .map(|attchmt| {
        let (prev_stage_tex, curr_stage_tex) = get_pipeline_stages_for_read(attchmt);
        prev_op_stage |= prev_stage_tex;
        current_op_stage |= curr_stage_tex;
        attchmt.barrier_prepare_attachment_for_shader_read()
      })
      .collect::<Vec<_>>();

    device.cmd_pipeline_barrier(
      command_buffer,
      // wait for previous use in:
      prev_op_stage,
      // before we: execute fragment shader / depth test
      current_op_stage,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &barriers,
    );
  }

  /// Most common layout transition between passes are attachment `read->write` or `write->read` or `write->write`.
  ///
  /// Util to wrap the barrier code to make the attachments
  /// **WRITEABLE IN FRAGMENT SHADER OR FOR DEPTH/STENCIL TEST**.
  pub unsafe fn cmd_transition_attachments_for_write_barrier(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    attachments: &mut [&mut VkTexture],
  ) {
    let mut prev_op_stage = vk::PipelineStageFlags::empty();
    let mut current_op_stage = vk::PipelineStageFlags::empty();

    let barriers = attachments
      .iter_mut()
      .map(|attchmt| {
        let (prev_stage_tex, curr_stage_tex) = get_pipeline_stages_for_write(attchmt);
        prev_op_stage |= prev_stage_tex;
        current_op_stage |= curr_stage_tex;
        attchmt.barrier_prepare_attachment_for_write()
      })
      .collect::<Vec<_>>();

    device.cmd_pipeline_barrier(
      command_buffer,
      // wait for previous use in:
      prev_op_stage,
      // before we:
      current_op_stage,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &barriers,
    );
  }
}

/// https://docs.vulkan.org/spec/latest/chapters/synchronization.html#synchronization-pipeline-stages-order
///
/// @return `(src_stage_mask/prev_op_stage, dst_stage_mask/current_op_stage)` depending on color/depth/stencil aspect.
fn get_pipeline_stages_for_read(
  texture: &VkTexture,
) -> (vk::PipelineStageFlags, vk::PipelineStageFlags) {
  if texture.is_color() {
    return (
      // wait for:
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, // only this as there is no read-after-read conflict!
      // before we:
      vk::PipelineStageFlags::FRAGMENT_SHADER,
    );
  }
  if texture.is_depth() || texture.is_depth_stencil() {
    // We do not know if previous/current passes used early depth stencil test, so both flags here. Suboptimal..
    return (
      // wait for:
      vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS, // also includes store ops
      // before we:
      vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
    );
  }

  panic!(
    "Could not determine layout transtion PipelineStageFlags for '{:?}'",
    texture.aspect_flags
  )
}

/// https://docs.vulkan.org/spec/latest/chapters/synchronization.html#synchronization-pipeline-stages-order
///
/// @return `(src_stage_mask/prev_op_stage, dst_stage_mask/current_op_stage)` depending on color/depth/stencil aspect.
fn get_pipeline_stages_for_write(
  texture: &VkTexture,
) -> (vk::PipelineStageFlags, vk::PipelineStageFlags) {
  if texture.is_color() {
    return (
      // wait for:
      // vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | // TODO [NOW] wait for last write
      vk::PipelineStageFlags::FRAGMENT_SHADER, // wait for last read
      // before we:
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
    );
  }
  if texture.is_depth() || texture.is_depth_stencil() {
    // We do not know if previous/current passes used early depth stencil test, so both flags here. Suboptimal..
    return (
      // wait for:
      // vk::PipelineStageFlags::FRAGMENT_SHADER | // TODO [NOW] wait for last read
      vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS, // write
      // before we:
      vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS, // also includes store ops
    );
  }

  panic!(
    "Could not determine layout transtion PipelineStageFlags for '{:?}'",
    texture.aspect_flags
  )
}
