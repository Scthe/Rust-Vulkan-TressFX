use ash;
use ash::version::DeviceV1_0;
use ash::vk::{self};

use crate::vk_utils::VkBuffer;

// https://vulkan-tutorial.com/Uniform_buffers/Descriptor_layout_and_buffer <3

/// Creates `$in_flight_frames` descriptor sets based on the provided layout
pub unsafe fn create_descriptor_set(
  device: &ash::Device,
  descriptor_pool: &vk::DescriptorPool,
  in_flight_frames: usize,
  descriptor_set_layout: &vk::DescriptorSetLayout,
) -> Vec<vk::DescriptorSet> {
  let mut set_layouts: Vec<vk::DescriptorSetLayout> = Vec::new();
  (0..in_flight_frames).for_each(|_| set_layouts.push(*descriptor_set_layout));

  let mut alloc_info = vk::DescriptorSetAllocateInfo::builder()
    .descriptor_pool(*descriptor_pool)
    .set_layouts(&set_layouts)
    .build();
  alloc_info.descriptor_set_count = in_flight_frames as u32;

  device
    .allocate_descriptor_sets(&alloc_info)
    .expect("Failed allocating descriptor sets")
}

pub unsafe fn bind_uniform_buffer_to_descriptor(
  device: &ash::Device,
  binding: u32,
  buffer: &VkBuffer,
  descriptor_set: &vk::DescriptorSet,
) {
  let buffer_info = vk::DescriptorBufferInfo::builder()
  .buffer(buffer.buffer)
  .offset(0)
  .range(vk::WHOLE_SIZE) // or buffer.size
  .build();

  let descriptor_binding = vk::WriteDescriptorSet::builder()
    .dst_set(*descriptor_set)
    .dst_binding(binding)
    .dst_array_element(0)
    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
    .buffer_info(&[buffer_info])
    .build();

  device.update_descriptor_sets(&[descriptor_binding], &[]);
}
