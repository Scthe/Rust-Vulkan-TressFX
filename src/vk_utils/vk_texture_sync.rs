use log::trace;

use ash;
use ash::vk;

use crate::config::Config;
use crate::vk_utils::create_image_barrier;

use super::VkMemoryResource;
use super::VkStorageResourceBarrier;
use super::VkTexture;
use super::WithSetupCmdBuffer;

// https://github.com/Tobski/simple_vulkan_synchronization/blob/main/thsvs_simpler_vulkan_synchronization.h
// https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples-(Legacy-synchronization-APIs)
// https://github.com/EmbarkStudios/kajiya/blob/main/crates/lib/kajiya-rg/src/graph.rs#L950

impl VkTexture {
  /// `VkImageCreateInfo.initialLayout` has to be `VK_IMAGE_LAYOUT_UNDEFINED` or `VK_IMAGE_LAYOUT_PREINITIALIZED`.
  /// Change this here.
  ///
  /// * https://vulkan.lunarg.com/doc/view/1.3.261.1/windows/1.3-extensions/vkspec.html#VUID-VkImageCreateInfo-initialLayout-00993
  pub(super) fn set_initial_image_layout(
    &mut self,
    with_setup_cb: &impl WithSetupCmdBuffer,
    new_layout: vk::ImageLayout,
  ) {
    // not terribly efficient, but usually part of init code so..
    #[allow(deprecated)]
    let barrier = VkStorageResourceBarrier::full_pipeline_stall();

    let vk_barrier = self.barrier_prepare_for_layout_transition(new_layout, barrier);
    let barriers = [vk_barrier];
    with_setup_cb.with_setup_cb(|device, cmd_buf| {
      unsafe {
        let dep = vk::DependencyInfo::builder().image_memory_barriers(&barriers);
        device.cmd_pipeline_barrier2(cmd_buf, &dep);
      };
    });
  }

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
  /// * `barrier` - previous op and op we will do
  pub(super) fn barrier_prepare_for_layout_transition(
    &mut self,
    new_layout: vk::ImageLayout,
    barrier: VkStorageResourceBarrier,
  ) -> vk::ImageMemoryBarrier2 {
    self.trace_log_layout_transition(new_layout, &barrier);

    let old_layout = self.layout;
    self.layout = new_layout;
    create_image_barrier(
      self.image,
      self.aspect_flags,
      old_layout,
      new_layout,
      barrier,
    )
  }

  pub(super) fn trace_log_layout_transition(
    &mut self,
    new_layout: vk::ImageLayout,
    barrier: &VkStorageResourceBarrier,
  ) {
    if Config::DEBUG_LAYOUT_TRANSITIONS {
      trace!(
        "VkTexture::LayoutTransition '{}' ({:?} -> {:?}), ||| BARRIER: {:?}",
        self.get_long_name(),
        self.layout,
        new_layout,
        barrier
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
    let mut barriers: Vec<vk::ImageMemoryBarrier2> = Vec::with_capacity(attachments.len());

    attachments.iter_mut().for_each(|attchmt| {
      let next_layout = get_layout_for_read(&attchmt);
      // skipped layout change if already in correct layout (read after read is always OK)
      if attchmt.layout != next_layout {
        let barrier = get_access_read(&attchmt);
        barriers.push(attchmt.barrier_prepare_for_layout_transition(next_layout, barrier));
      }
    });

    let dep = vk::DependencyInfo::builder().image_memory_barriers(&barriers);
    device.cmd_pipeline_barrier2(command_buffer, &dep);
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
    let barriers = attachments
      .iter_mut()
      .map(|attchmt| {
        let next_layout = get_layout_for_write(attchmt);
        let barrier = get_access_write(attchmt);
        attchmt.barrier_prepare_for_layout_transition(next_layout, barrier)
      })
      .collect::<Vec<_>>();

    let dep = vk::DependencyInfo::builder().image_memory_barriers(&barriers);
    device.cmd_pipeline_barrier2(command_buffer, &dep);
  }
}

/// @returns `true` if layout is for writes, `false` if it is for reads
fn is_layout_write(layout: vk::ImageLayout) -> bool {
  match layout {
    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
    | vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL
    | vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL => return false,
    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
    | vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
    | vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL => return true,
    _ => panic!(
      "Could not determine if layout '{:?}' is for write or read",
      layout
    ),
  }
}

fn get_layout_for_read(tex: &VkTexture) -> vk::ImageLayout {
  if tex.is_color() {
    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
  } else if tex.is_depth_stencil() {
    vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL
  } else if tex.is_depth() {
    vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL
  } else {
    panic!("Tried to transition texture {} for shader write, but it's neither color or depth-stencil texture.", tex.get_long_name());
  }
}

/// read-after-write
fn get_access_read(tex: &VkTexture) -> VkStorageResourceBarrier {
  // TODO Vulkan tools complain cause validation layer has bug. Need to update validation layer.
  // https://stackoverflow.com/questions/75743040/vulkan-sync-hazard-read-after-write-despite-full-pipeline-barrier-between-opera
  let mut barrier = VkStorageResourceBarrier::empty();

  // we will sample in fragment shader
  barrier.next_op.0 = vk::PipelineStageFlags2::FRAGMENT_SHADER;
  barrier.next_op.1 = vk::AccessFlags2::SHADER_SAMPLED_READ;

  // previous op was write. Either color attachment or depth/stencil early/late tests
  if tex.is_color() {
    barrier.previous_op.0 = vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT;
    barrier.previous_op.1 = vk::AccessFlags2::COLOR_ATTACHMENT_WRITE;
  } else if tex.is_depth_stencil() || tex.is_depth() {
    // also includes store ops
    barrier.previous_op.0 =
      vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS | vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS;
    barrier.previous_op.1 = vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE;
  } else {
    panic!("Tried to transition texture {} for shader read, but it's neither color or depth-stencil texture.", tex.get_long_name());
  }

  barrier
}

fn get_layout_for_write(tex: &VkTexture) -> vk::ImageLayout {
  if tex.is_color() {
    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
  } else if tex.is_depth_stencil() {
    vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
  } else if tex.is_depth() {
    vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL
  } else {
    panic!("Tried to transition texture {} for shader write, but it's neither color or depth-stencil texture.", tex.get_long_name());
  }
}

/// * write-after-write - memory dependency, requires `PipelineStageFlags2+AccessFlags2`
/// * write-after-read - execution dependency, only `PipelineStageFlags2`
fn get_access_write(tex: &VkTexture) -> VkStorageResourceBarrier {
  let is_write_after_write = is_layout_write(tex.layout);
  let mut barrier = VkStorageResourceBarrier::empty();
  let p_stages_depth =
    vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS | vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS; // also includes store ops
  let access_depth_rw = vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
    | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE;

  if tex.is_color() {
    barrier.next_op.0 = vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT;
    if is_write_after_write {
      barrier.previous_op.0 = vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT; // wait for last write
      barrier.previous_op.1 = vk::AccessFlags2::COLOR_ATTACHMENT_WRITE;
      barrier.next_op.1 = vk::AccessFlags2::COLOR_ATTACHMENT_WRITE;
    } else {
      barrier.previous_op.0 = vk::PipelineStageFlags2::FRAGMENT_SHADER; // wait for last read
    }
  } else if tex.is_depth_stencil() || tex.is_depth() {
    barrier.next_op.0 = p_stages_depth;
    if is_write_after_write {
      barrier.previous_op.0 = p_stages_depth; // wait for last write
      barrier.previous_op.1 = vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE;
      barrier.next_op.1 = access_depth_rw;
    } else {
      barrier.previous_op.0 = vk::PipelineStageFlags2::FRAGMENT_SHADER; // wait for last read
    }
  } else {
    panic!("Tried to transition texture {} for shader write, but it's neither color or depth-stencil texture.", tex.get_long_name());
  }

  barrier
}
