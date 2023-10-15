use ash;
use ash::vk;

pub fn create_color_attachment(
  attachment_idx: u32,
  image_format: vk::Format,
  load_op: vk::AttachmentLoadOp,
  store_op: vk::AttachmentStoreOp,
  final_layout: vk::ImageLayout,
) -> (vk::AttachmentDescription, vk::AttachmentReference) {
  let attachment = vk::AttachmentDescription::builder()
  .format(image_format)
  .samples(vk::SampleCountFlags::TYPE_1) // single sampled
  .load_op(load_op)
  .store_op(store_op)
  // .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
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
) -> (vk::AttachmentDescription, vk::AttachmentReference) {
  let attachment = vk::AttachmentDescription::builder()
  .format(image_format)
  .samples(vk::SampleCountFlags::TYPE_1) // single sampled
  .load_op(depth_load_op)
  .store_op(depth_store_op)
  .stencil_load_op(stencil_load_op)
  .stencil_store_op(stencil_store_op)
  // .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
  .final_layout(final_layout)
  .build();

  let attachment_reference = vk::AttachmentReference {
    attachment: attachment_idx,
    layout: final_layout,
  };

  (attachment, attachment_reference)
}
