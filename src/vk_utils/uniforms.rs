use ash;
use ash::extensions::khr::PushDescriptor;
use ash::vk::{self};

use super::VkTexture;
use crate::vk_utils::VkBuffer;

/*
////////////////////////////////////////
/// NOTES FOR DESCRIPTOR SETS
/// Obsolet as we use VK_KHR_push_descriptor, but still good to know
////////////////////////////////////////

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

////////////////////////////////
/// Layout utils
////////////////////////////////

/// Create layout for a single uniform buffer object.
/// That layout will be one of layouts gathered in DescriptorSetLayout.
pub fn create_ubo_binding(
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

/// Create layout for a single texture/sampler object.
/// That layout will be one of layouts gathered in DescriptorSetLayout.
pub fn create_texture_binding(
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

/// Create layout for a single shader storage buffer object (SSBO).
/// That layout will be one of layouts gathered in DescriptorSetLayout.
pub fn create_ssbo_binding(
  binding: u32,
  stage_flags: vk::ShaderStageFlags,
) -> vk::DescriptorSetLayoutBinding {
  vk::DescriptorSetLayoutBinding::builder()
    .binding(binding)
    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
    .descriptor_count(1)
    .stage_flags(stage_flags)
    .build()
}

pub fn create_push_descriptor_layout(
  device: &ash::Device,
  bindings: Vec<vk::DescriptorSetLayoutBinding>,
) -> vk::DescriptorSetLayout {
  let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
    .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR)
    .bindings(&bindings)
    .build();

  unsafe {
    device
      .create_descriptor_set_layout(&create_info, None)
      .expect("Failed to create DescriptorSetLayout")
  }
}

////////////////////////////////
/// Resource binding
////////////////////////////////

/// Pack stuff into struct to make it more palatable as param
pub struct ResouceBinder<'a> {
  pub push_descriptor: &'a PushDescriptor,
  pub command_buffer: vk::CommandBuffer,
  pub pipeline_layout: vk::PipelineLayout,
}

pub enum BindableBufferUsage {
  UBO,
  SSBO,
}

fn get_buffer_descriptor_type(buf_type: &BindableBufferUsage) -> vk::DescriptorType {
  match buf_type {
    BindableBufferUsage::UBO => vk::DescriptorType::UNIFORM_BUFFER,
    BindableBufferUsage::SSBO => vk::DescriptorType::STORAGE_BUFFER,
  }
}

pub enum BindableResource<'a> {
  Buffer {
    usage: BindableBufferUsage,
    binding: u32,
    buffer: &'a VkBuffer,
  },
  Texture {
    binding: u32,
    texture: &'a VkTexture,
    image_view: Option<vk::ImageView>,
    sampler: vk::Sampler,
  },
}

pub unsafe fn bind_resources_to_descriptors(
  binder: &ResouceBinder,
  descriptor_set: u32,
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
        BindableResource::Buffer {
          binding,
          buffer,
          usage,
        } => {
          buffer_infos.push(vk::DescriptorBufferInfo {
            buffer: buffer.buffer,
            offset: 0,
            range: vk::WHOLE_SIZE, // or buffer.size
          });
          let data_slice = &buffer_infos[(buffer_infos.len() - 1)..buffer_infos.len()];
          vk::WriteDescriptorSet::builder()
            .dst_binding(*binding)
            .dst_array_element(0)
            .descriptor_type(get_buffer_descriptor_type(usage))
            .buffer_info(data_slice)
            .build()
        }
        BindableResource::Texture {
          binding,
          texture,
          sampler,
          image_view,
        } => {
          let iv: vk::ImageView = image_view.unwrap_or(texture.image_view());
          image_infos.push(vk::DescriptorImageInfo {
            image_layout: texture.layout,
            image_view: iv,
            sampler: *sampler,
          });
          let data_slice = &image_infos[(image_infos.len() - 1)..image_infos.len()];
          vk::WriteDescriptorSet::builder()
            .dst_binding(*binding)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(data_slice)
            .build()
        }
      }
    })
    .collect();

  binder.push_descriptor.cmd_push_descriptor_set(
    binder.command_buffer,
    vk::PipelineBindPoint::GRAPHICS,
    binder.pipeline_layout,
    descriptor_set,
    next_descriptors.as_slice(),
  );
}
