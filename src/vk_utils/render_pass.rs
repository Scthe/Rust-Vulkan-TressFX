use ash;
use ash::vk;

/// Raw Vulkan objects used to create vk::RenderPass
pub type AttachmentDefinition = (vk::AttachmentDescription, vk::AttachmentReference);

///
/// - presentable - `true` if image is rendered to window framebuffer. `false` if it's user-created texture
pub fn create_color_attachment(
  attachment_idx: u32,
  image_format: vk::Format,
  load_op: vk::AttachmentLoadOp,
  store_op: vk::AttachmentStoreOp,
  presentable: bool,
) -> AttachmentDefinition {
  let final_layout = if presentable {
    vk::ImageLayout::PRESENT_SRC_KHR
  } else {
    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
  };

  let attachment = vk::AttachmentDescription::builder()
  .format(image_format)
  .samples(vk::SampleCountFlags::TYPE_1) // single sampled
  .load_op(load_op)
  .store_op(store_op)
  .initial_layout(final_layout)
  .final_layout(final_layout)
  .build();

  let attachment_reference = vk::AttachmentReference {
    attachment: attachment_idx,
    layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
  };

  (attachment, attachment_reference)
}

pub fn create_depth_stencil_attachment(
  attachment_idx: u32,
  image_format: vk::Format,
  depth_load_op: vk::AttachmentLoadOp,
  depth_store_op: vk::AttachmentStoreOp,
  stencil_load_op: vk::AttachmentLoadOp,
  stencil_store_op: vk::AttachmentStoreOp,
  final_layout: vk::ImageLayout,
) -> AttachmentDefinition {
  let attachment = vk::AttachmentDescription::builder()
  .format(image_format)
  .samples(vk::SampleCountFlags::TYPE_1) // single sampled
  .load_op(depth_load_op)
  .store_op(depth_store_op)
  .stencil_load_op(stencil_load_op)
  .stencil_store_op(stencil_store_op)
  .initial_layout(final_layout)
  .final_layout(final_layout)
  .build();

  let attachment_reference = vk::AttachmentReference {
    attachment: attachment_idx,
    layout: final_layout,
  };

  (attachment, attachment_reference)
}

pub unsafe fn create_render_pass_from_attachments(
  device: &ash::Device,
  depth: Option<AttachmentDefinition>,
  colors: &[AttachmentDefinition],
) -> vk::RenderPass {
  // TODO [LOW] this fn can implicitly change layouts by itself - use instead of `cmd_transition_attachments_for_write_barrier`?
  //      Though that would require knowledge of previous layout, which we only know at execution state
  let mut all_attachment_descs = Vec::<vk::AttachmentDescription>::with_capacity(colors.len() + 1);
  let mut src_stage_mask = vk::PipelineStageFlags::empty();
  let mut dst_stage_mask = vk::PipelineStageFlags::empty();
  let mut dst_access_mask = vk::AccessFlags::empty();

  let color_refs = colors.iter().map(|a| a.1).collect::<Vec<_>>();
  let mut subpass = vk::SubpassDescription::builder()
    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
    .color_attachments(&color_refs)
    // depth added below
    .build();

  // depth
  if let Some(a_ds) = depth {
    src_stage_mask = src_stage_mask
      | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
      | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
    dst_stage_mask = dst_stage_mask
      | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
      | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS;
    dst_access_mask = dst_access_mask | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
    all_attachment_descs.push(a_ds.0);
    subpass.p_depth_stencil_attachment = &a_ds.1 as *const vk::AttachmentReference;
  }

  // colors
  if colors.len() > 0 {
    src_stage_mask = src_stage_mask | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
    dst_stage_mask = dst_stage_mask | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
    dst_access_mask = dst_access_mask | vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
  }
  colors.iter().for_each(|a| all_attachment_descs.push(a.0));

  // probably does not matter if we have only 1 subpass?
  let dependencies = vk::SubpassDependency::builder()
    .src_subpass(vk::SUBPASS_EXTERNAL)
    .dst_subpass(0)
    .src_stage_mask(src_stage_mask)
    .src_access_mask(vk::AccessFlags::empty())
    .dst_stage_mask(dst_stage_mask)
    .dst_access_mask(dst_access_mask)
    .build();

  let create_info = vk::RenderPassCreateInfo::builder()
    .dependencies(&[dependencies])
    .attachments(&all_attachment_descs)
    .subpasses(&[subpass])
    .build();
  device
    .create_render_pass(&create_info, None)
    .expect("Failed creating render pass")
}
