use log::trace;

use ash;
use ash::vk;

use crate::vk_utils::create_image_barrier;

use super::VkMemoryResource;
use super::VkTexture;

/*
TODO create wrapped `cmd_texture_layout_barrier_read(&[&VkTexture]);` or `cmd_transition_attachment_for_read(&[&VkTexture])`
     it auto checks if there are color/depth textures and adjust the PipelineStageFlags
TODO https://github.com/Tobski/simple_vulkan_synchronization/blob/main/thsvs_simpler_vulkan_synchronization.h
*/

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
  /// TODO [???] return Option if layout already matches? What if we want barrier with no layout change (Read-After-Read?)
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

  pub fn barrier_prepare_attachment_for_shader_read(&mut self) -> vk::ImageMemoryBarrier {
    if self.is_color() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE, // prev op
        vk::AccessFlags::SHADER_READ | vk::AccessFlags::INPUT_ATTACHMENT_READ, // our op
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

  pub fn barrier_prepare_attachment_for_write(&mut self) -> vk::ImageMemoryBarrier {
    if self.is_color() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::SHADER_READ,            // prev op
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE, // our op
      )
    } else if self.is_depth_stencil() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ, // prev op
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, // our op
      )
    } else if self.is_depth() {
      self.barrier_prepare_for_layout_transition(
        vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ, // prev op
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, // our op
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
}
