use ash::vk;
use ash::{self};

use crate::vk_utils::debug::{set_buffer_debug_label, set_texture_debug_label};
use crate::vk_utils::{VkBuffer, VkMemoryResource, VkTexture};

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
    let buffer = VkBuffer::empty(
      &self.allocator,
      self.device.queue_family_index,
      name,
      size,
      usage,
      mappable,
    );
    self.assign_buffer_debug_label(&buffer);
    buffer
  }

  pub fn create_buffer_from_data(
    &self,
    name: String,
    bytes: &[u8],
    usage: vk::BufferUsageFlags,
  ) -> VkBuffer {
    let buffer = VkBuffer::from_data(
      &self.allocator,
      self.device.queue_family_index,
      self,
      name,
      bytes,
      usage,
    );
    self.assign_buffer_debug_label(&buffer);
    buffer
  }

  fn assign_buffer_debug_label(&self, buffer: &VkBuffer) {
    unsafe {
      set_buffer_debug_label(
        &self.debug_utils_loader,
        &self.device.device.handle(),
        buffer.buffer,
        &buffer.get_name(),
      )
    };
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
    let tex = VkTexture::empty(
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
    );
    self.assign_texture_debug_label(&tex);
    tex
  }

  pub fn create_texture_from_file(&self, path: &std::path::Path, format: vk::Format) -> VkTexture {
    let tex = VkTexture::from_file(&self.vk_device(), &self.allocator, self, path, format);
    self.assign_texture_debug_label(&tex);
    tex
  }

  pub fn create_texture_from_data(
    &self,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    data_bytes: &Vec<u8>,
  ) -> VkTexture {
    let tex = VkTexture::from_data(
      &self.vk_device(),
      &self.allocator,
      self,
      name,
      size,
      format,
      data_bytes,
    );
    self.assign_texture_debug_label(&tex);
    tex
  }

  fn assign_texture_debug_label(&self, tex: &VkTexture) {
    unsafe {
      set_texture_debug_label(
        &self.debug_utils_loader,
        &self.device.device.handle(),
        tex.image,
        &tex.get_name(),
      )
    };
  }
}
