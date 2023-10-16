use ash;
use ash::vk;

/* TODO remove?
pub fn create_image_barrier(
  image: vk::Image,
  aspect_mask: vk::ImageAspectFlags,
  src_access_mask: vk::AccessFlags,
  dst_access_mask: vk::AccessFlags,
) -> vk::ImageMemoryBarrier {
  vk::ImageMemoryBarrier::builder()
    .src_access_mask(src_access_mask)
    .dst_access_mask(dst_access_mask)
    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
    .image(image)
    .subresource_range(vk::ImageSubresourceRange {
      aspect_mask,
      base_mip_level: 0,
      level_count: 1, // vk::REMAINING_MIP_LEVELS
      base_array_layer: 0,
      layer_count: 1, // vk::REMAINING_ARRAY_LAYERS
    })
    .build()
}
*/

pub fn create_image_barrier(
  image: vk::Image,
  aspect_mask: vk::ImageAspectFlags,
  old_layout: vk::ImageLayout,
  new_layout: vk::ImageLayout,
  src_access_mask: vk::AccessFlags,
  dst_access_mask: vk::AccessFlags,
) -> vk::ImageMemoryBarrier {
  vk::ImageMemoryBarrier::builder()
    .old_layout(old_layout)
    .new_layout(new_layout)
    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
    .image(image)
    .subresource_range(vk::ImageSubresourceRange {
      aspect_mask,
      base_mip_level: 0,
      level_count: 1, // vk::REMAINING_MIP_LEVELS
      base_array_layer: 0,
      layer_count: 1, // vk::REMAINING_ARRAY_LAYERS
    })
    .build()
}
