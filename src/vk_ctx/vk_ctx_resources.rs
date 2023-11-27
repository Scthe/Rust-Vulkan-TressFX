use ash::vk;
use ash::{self};

use crate::utils::get_attachment_name;
use crate::vk_utils::debug::{set_buffer_debug_label, set_texture_debug_label};
use crate::vk_utils::{
  get_image_aspect_from_format, VkBuffer, VkMemoryPreference, VkMemoryResource, VkTexture,
};

use super::*;

impl VkCtx {
  // buffers
  pub fn create_buffer_empty(
    &self,
    name: String,
    size: usize,
    usage: vk::BufferUsageFlags,
    memory_pref: VkMemoryPreference,
  ) -> VkBuffer {
    let buffer = VkBuffer::empty(
      &self.allocator,
      self.device.queue_family_index,
      name,
      size,
      usage,
      memory_pref,
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
    self.with_debug_loader(|debug_utils_loader| {
      unsafe {
        set_buffer_debug_label(
          &debug_utils_loader,
          &self.device.device.handle(),
          buffer.buffer,
          &buffer.get_name(),
        )
      };
    });
  }

  // textures
  pub fn create_texture_empty(
    &self,
    name: String,
    size: vk::Extent2D,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    memory_pref: VkMemoryPreference,
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
      memory_pref,
      initial_layout,
    );
    self.assign_texture_debug_label(&tex);
    tex
  }

  pub fn create_attachment<PassType>(
    &self,
    name: &str,
    frame_id: usize,
    format: vk::Format,
    size: vk::Extent2D,
  ) -> VkTexture {
    let aspect = get_image_aspect_from_format(format);
    let mut usage_flags = vk::ImageUsageFlags::SAMPLED;
    let mut initial_layout = vk::ImageLayout::PREINITIALIZED;

    if aspect == (vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL) {
      usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
      initial_layout = vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL;
    }
    if aspect == vk::ImageAspectFlags::DEPTH {
      usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
      initial_layout = vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL;
    }
    if aspect == vk::ImageAspectFlags::COLOR {
      usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
      initial_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
    }
    assert!(
      // we have not changed from defaults - invalid!
      initial_layout != vk::ImageLayout::PREINITIALIZED,
      "Could not determine create_attachment properties for {:?} ({:?})",
      format,
      aspect
    );

    self.create_texture_empty(
      get_attachment_name::<PassType>(name, frame_id),
      size,
      format,
      vk::ImageTiling::OPTIMAL,
      usage_flags,
      VkMemoryPreference::GpuOnly,
      initial_layout,
    )
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
    self.with_debug_loader(|debug_utils_loader| {
      unsafe {
        set_texture_debug_label(
          &debug_utils_loader,
          &self.device.device.handle(),
          tex.image,
          &tex.get_name(),
        )
      };
    });
  }
}
