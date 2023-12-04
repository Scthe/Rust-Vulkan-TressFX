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

/// fence - used to wait on CPU till it is signaled
pub fn create_fence(device: &ash::Device) -> vk::Fence {
  let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
  unsafe {
    device
      .create_fence(&create_info, None)
      .expect("Failed to create fence")
  }
}

pub fn create_fences(device: &ash::Device, count: usize) -> Vec<vk::Fence> {
  let mut result = Vec::<vk::Fence>::with_capacity(count);
  for _ in 0..count {
    let obj = create_fence(device);
    result.push(obj);
  }
  result
}

/// semaphore - used to synchronize work between queues (including present op.)
pub fn create_semaphore(device: &ash::Device) -> vk::Semaphore {
  let semaphore_create_info = vk::SemaphoreCreateInfo::builder()
    .flags(vk::SemaphoreCreateFlags::empty())
    .build();
  unsafe {
    device
      .create_semaphore(&semaphore_create_info, None)
      .expect("Failed to create semaphore")
  }
}

pub fn create_semaphores(device: &ash::Device, count: usize) -> Vec<vk::Semaphore> {
  let mut result = Vec::<vk::Semaphore>::with_capacity(count);
  for _ in 0..count {
    let obj = create_semaphore(device);
    result.push(obj);
  }
  result
}

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
#[deprecated(note = "Extremely suboptimal performance")]
pub unsafe fn execute_full_pipeline_barrier(
  device: &ash::Device,
  command_buffer: vk::CommandBuffer,
) -> () {
  #[allow(deprecated)]
  let mem_barrier = VkStorageResourceBarrier::full_pipeline_stall();
  cmd_storage_resource_barrier(device, command_buffer, mem_barrier);
}

/// Barrier mostly used for SSBO and storage images. No layout transitions etc.
#[derive(Debug)]
pub struct VkStorageResourceBarrier {
  /// wait for previous use in
  pub previous_op: (vk::PipelineStageFlags2, vk::AccessFlags2),
  /// before we
  pub next_op: (vk::PipelineStageFlags2, vk::AccessFlags2),
}

impl VkStorageResourceBarrier {
  pub fn empty() -> Self {
    Self {
      previous_op: (vk::PipelineStageFlags2::empty(), vk::AccessFlags2::empty()),
      next_op: (vk::PipelineStageFlags2::empty(), vk::AccessFlags2::empty()),
    }
  }

  /// DANGEROUS! WILL STALL EVERYTHING. WILL FLUSH AND INVALIDATE ALL CACHES.
  #[deprecated(note = "Extremely suboptimal performance")]
  pub fn full_pipeline_stall() -> Self {
    let mut barrier = VkStorageResourceBarrier::empty();
    barrier.previous_op.0 = vk::PipelineStageFlags2::ALL_COMMANDS;
    barrier.previous_op.1 = vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE;
    barrier.next_op.0 = vk::PipelineStageFlags2::ALL_COMMANDS;
    barrier.next_op.1 = vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE;
    barrier
  }
}

/// Barrier mostly used for SSBO and storage images. No layout transitions etc.
pub unsafe fn cmd_storage_resource_barrier(
  device: &ash::Device,
  command_buffer: vk::CommandBuffer,
  barrier: VkStorageResourceBarrier,
) {
  let mem_barrier = vk::MemoryBarrier2::builder()
    // wait for previous use in:
    .src_stage_mask(barrier.previous_op.0)
    .src_access_mask(barrier.previous_op.1)
    // before we:
    .dst_stage_mask(barrier.next_op.0)
    .dst_access_mask(barrier.next_op.1)
    .build();
  let dependency_info = vk::DependencyInfo::builder()
    .memory_barriers(&[mem_barrier])
    .build();

  device.cmd_pipeline_barrier2(command_buffer, &dependency_info);
}

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkImageMemoryBarrier2.html
pub fn create_image_barrier(
  image: vk::Image,
  aspect_mask: vk::ImageAspectFlags,
  old_layout: vk::ImageLayout,
  new_layout: vk::ImageLayout,
  barrier: VkStorageResourceBarrier,
) -> vk::ImageMemoryBarrier2 {
  vk::ImageMemoryBarrier2::builder()
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
    // wait for previous use in:
    .src_stage_mask(barrier.previous_op.0)
    .src_access_mask(barrier.previous_op.1)
    // before we:
    .dst_stage_mask(barrier.next_op.0)
    .dst_access_mask(barrier.next_op.1)
    .build()
}
