use ash;
use ash::vk::{self};

use crate::vk_utils::VkBuffer;

use super::VkTexture;

/*
https://vulkan-tutorial.com/Uniform_buffers/Descriptor_layout_and_buffer <3

You cannot bind a single shader resource to a buffer/texture. You can only bind a group
of resources as descriptor sets.

Steps:
  1. Create descriptor pool. Specify how many descriptors will be allocated
  2. Create descriptor set(s). This are connected to each shader. Each descriptor set
     contains some number of uniform buffers/textures, each assigned a `binding`.
  3. Connect the real data buffer to a (descriptor_set, binding) using `vkUpdateDescriptorSets`.
  4. Bind the descriptor sets before draw call: `vkCmdBindDescriptorSets`.

On a lot of hardware only 4 descriptor sets can be bound to a single pipeline.
E.g. 1st descriptor is for global shared config data. 2nd is for per-model data etc.

In shader:
// shared by all shaders - one global descriptor set bound for every pass/draw call.
layout(set=0, binding=0) uniform GlobalConfigData;
// model data (struct for material data etc.)
layout(set=1, binding=0) uniform ModelData;
// model data (diffuse texture)
layout(set=1, binding=1) sampler Texture2D tex_diff;

DescriptorSetLayout is required during:
- creating descriptor set so we can bind the data
- creating rendering pipeline
*/

pub fn create_descriptor_pool(
  device: &ash::Device,
  descriptor_types: &[vk::DescriptorType],
  frames_in_flight: u32,
  set_count: u32,
) -> vk::DescriptorPool {
  let descriptor_pool_size: Vec<vk::DescriptorPoolSize> = descriptor_types
    .iter()
    .map(|descriptor_type| {
      vk::DescriptorPoolSize::builder()
        .ty(*descriptor_type)
        .descriptor_count(frames_in_flight)
        .build()
    })
    .collect();
  let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
    .pool_sizes(&descriptor_pool_size[..]) // creat slice
    // TODO this fn should be rewritten. Or create pool per-usage (per descriptor set TBH)?
    .max_sets(frames_in_flight * set_count)
    .build();
  unsafe {
    device
      .create_descriptor_pool(&descriptor_pool_create_info, None)
      .expect("Failed creating descriptor pool for 1st time")
  }
}

/// Creates `$in_flight_frames` descriptor sets based on the provided layout
pub unsafe fn create_descriptor_sets(
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

////////////////////////////////
/// Layout utils
////////////////////////////////

pub fn create_ubo_layout(
  binding: u32,
  stage_flags: vk::ShaderStageFlags,
) -> vk::DescriptorSetLayoutBinding {
  vk::DescriptorSetLayoutBinding::builder()
    .binding(binding)
    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
    .descriptor_count(1)
    .stage_flags(stage_flags)
    .build()
}

pub fn create_texture_layout(
  binding: u32,
  stage_flags: vk::ShaderStageFlags,
) -> vk::DescriptorSetLayoutBinding {
  vk::DescriptorSetLayoutBinding::builder()
    .binding(binding)
    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
    .descriptor_count(1)
    .stage_flags(stage_flags)
    .build()
}

////////////////////////////////
/// Resource binding
////////////////////////////////

pub enum BindableResource<'a> {
  Uniform {
    descriptor_set: vk::DescriptorSet,
    binding: u32,
    buffer: &'a VkBuffer,
  },
  Texture {
    descriptor_set: vk::DescriptorSet,
    binding: u32,
    texture: &'a VkTexture,
    sampler: vk::Sampler,
  },
}

pub unsafe fn bind_resources_to_descriptors(
  device: &ash::Device,
  resources_to_bind: &[BindableResource],
) {
  // Used to ensure lifetime of these vk::* objects till the end of this fn ().
  // Since vk::WriteDescriptorSet has POINTERS to data, we need to have these pointers
  // reference alive right thing.
  let mut buffer_infos: Vec<vk::DescriptorBufferInfo> = Vec::with_capacity(resources_to_bind.len());
  let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(resources_to_bind.len());

  let next_descriptors: Vec<vk::WriteDescriptorSet> = resources_to_bind
    .iter()
    .map(|resource| {
      match resource {
        BindableResource::Uniform {
          descriptor_set,
          binding,
          buffer,
        } => {
          buffer_infos.push(vk::DescriptorBufferInfo {
            buffer: buffer.buffer,
            offset: 0,
            range: vk::WHOLE_SIZE, // or buffer.size
          });
          let data_slice = &buffer_infos[(buffer_infos.len() - 1)..buffer_infos.len()];
          vk::WriteDescriptorSet::builder()
            .dst_set(*descriptor_set)
            .dst_binding(*binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(data_slice)
            .build()
        }
        BindableResource::Texture {
          descriptor_set,
          binding,
          texture,
          sampler,
        } => {
          image_infos.push(vk::DescriptorImageInfo {
            image_layout: texture.layout,
            image_view: texture.image_view(),
            sampler: *sampler,
          });
          let data_slice = &image_infos[(image_infos.len() - 1)..image_infos.len()];
          vk::WriteDescriptorSet::builder()
            .dst_set(*descriptor_set)
            .dst_binding(*binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(data_slice)
            .build()
        }
      }
    })
    .collect();

  device.update_descriptor_sets(next_descriptors.as_ref(), &[]);
}
