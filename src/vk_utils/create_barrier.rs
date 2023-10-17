use ash;
use ash::vk;

/*
https://docs.vulkan.org/spec/latest/chapters/synchronization.html#synchronization-global-memory-barriers
https://www.reddit.com/r/vulkan/comments/8y5g6g/usage_of_vkevents/
https://github.com/SaschaWillems/Vulkan/blob/master/examples/deferred/deferred.cpp#L447
https://gpuopen.com/learn/vulkan-barriers-explained/
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
    .src_access_mask(src_access_mask)
    .dst_access_mask(dst_access_mask)
    .subresource_range(vk::ImageSubresourceRange {
      aspect_mask,
      base_mip_level: 0,
      level_count: 1, // vk::REMAINING_MIP_LEVELS
      base_array_layer: 0,
      layer_count: 1, // vk::REMAINING_ARRAY_LAYERS
    })
    .build()
}
