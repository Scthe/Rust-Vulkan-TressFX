use ash::vk;
use ash::{self};

use crate::vk_utils::{VkBuffer, VkTexture};

use super::*;

impl VkCtx {
  // buffers
  pub fn create_buffer_empty(
    &self,
    name: String,
    size: usize,
    usage: vk::BufferUsageFlags,
    mappable: bool,
  ) -> VkBuffer {
    VkBuffer::empty(
      &self.allocator,
      self.device.queue_family_index,
      name,
      size,
      usage,
      mappable,
    )
  }

  pub fn create_buffer_from_data(
    &self,
    name: String,
    bytes: &[u8],
    usage: vk::BufferUsageFlags,
  ) -> VkBuffer {
    VkBuffer::from_data(
      &self.allocator,
      self.device.queue_family_index,
      self,
      name,
      bytes,
      usage,
    )
  }

  // textures
  pub fn create_texture_empty(
    &self,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    allocation_flags: vk::MemoryPropertyFlags,
    initial_layout: vk::ImageLayout,
  ) -> VkTexture {
    VkTexture::empty(
      &self.vk_device(),
      &self.allocator,
      self,
      name,
      size,
      format,
      tiling,
      usage,
      allocation_flags,
      initial_layout,
    )
  }

  pub fn create_texture_from_file(&self, path: &std::path::Path, format: vk::Format) -> VkTexture {
    VkTexture::from_file(&self.vk_device(), &self.allocator, self, path, format)
  }

  pub fn create_texture_from_data(
    &self,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    data_bytes: &Vec<u8>,
  ) -> VkTexture {
    VkTexture::from_data(
      &self.vk_device(),
      &self.allocator,
      self,
      name,
      size,
      format,
      data_bytes,
    )
  }
}
