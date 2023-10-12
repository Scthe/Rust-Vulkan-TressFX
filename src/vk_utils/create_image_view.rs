use ash::vk;

pub fn create_image_view(
  device: &ash::Device,
  image: vk::Image,
  image_format: vk::Format,
  aspect_mask_flags: vk::ImageAspectFlags,
  // base_mip_level: u32,
  // mip_level_count: u32,
) -> vk::ImageView {
  let subresource_range = vk::ImageSubresourceRange::builder()
    .aspect_mask(aspect_mask_flags)
    .base_array_layer(0)
    .layer_count(1)
    .base_mip_level(0) // base_mip_level
    .level_count(1) // mip_level_count
    .build();

  let create_info = vk::ImageViewCreateInfo::builder()
    .image(image)
    .view_type(vk::ImageViewType::TYPE_2D)
    .format(image_format)
    .subresource_range(subresource_range)
    .build();

  unsafe {
    device
      .create_image_view(&create_info, None)
      .expect("Failed creating image view")
  }
}
