use ash;
use ash::vk;

/*
https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples
https://docs.vulkan.org/spec/latest/chapters/synchronization.html#synchronization-global-memory-barriers
https://docs.vulkan.org/spec/latest/chapters/synchronization.html#synchronization-pipeline-stages-order - pipeline stages implicit order
https://www.reddit.com/r/vulkan/comments/8y5g6g/usage_of_vkevents/
https://github.com/SaschaWillems/Vulkan/blob/master/examples/deferred/deferred.cpp#L447
https://gpuopen.com/learn/vulkan-barriers-explained/
https://www.youtube.com/watch?v=JvAIdtAZnAw

libs
  - https://github.com/Tobski/simple_vulkan_synchronization/blob/main/thsvs_simpler_vulkan_synchronization.h
  - https://github.com/h3r2tic/vk-sync-rs/blob/master/src/lib.rs
*/

#[allow(dead_code)]
/// You should **ONLY USE THIS FOR DEBUGGING** - this is not something
/// that should ever ship in real code, this will flush
/// and invalidate all caches and stall everything, it is a tool
/// not to be used lightly!
///
/// That said, it can be really handy if you think you have a race
/// condition in your app and you just want to serialize everything
/// so you can debug it.
///
/// https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples#full-pipeline-barrier
pub unsafe fn execute_full_pipeline_barrier(
  device: &ash::Device,
  command_buffer: vk::CommandBuffer,
) -> () {
  let mem_barrier = vk::MemoryBarrier2::builder()
    .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
    .src_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
    .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
    .dst_access_mask(vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE)
    .build();
  let dependency_info = vk::DependencyInfo::builder()
    .memory_barriers(&[mem_barrier])
    .build();

  device.cmd_pipeline_barrier2(command_buffer, &dependency_info);
}

/// Barrier for all types of resources (both buffer and image)
/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkMemoryBarrier.html
/// TODO [HIGH] is global barrier optimal? Or maybe use fine grained buffer/image barriers instead?
pub fn create_global_barrier(
  src_access_mask: vk::AccessFlags,
  dst_access_mask: vk::AccessFlags,
) -> vk::MemoryBarrier {
  vk::MemoryBarrier::builder()
    .src_access_mask(src_access_mask)
    .dst_access_mask(dst_access_mask)
    .build()
}

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkImageMemoryBarrier.html
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
