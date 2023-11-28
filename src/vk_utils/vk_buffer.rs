use ash::vk;
use vma::Alloc;

use crate::vk_utils::get_persistently_mapped_pointer;

use super::{
  determine_gpu_allocation_info, MemoryMapPointer, VkMemoryPreference, VkMemoryResource,
  WithSetupCmdBuffer,
};

/*
https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/quick_start.html
https://github.com/expenses/vulkan-base/blob/main/ash-helpers/src/lib.rs

https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/choosing_memory_type.html
If you want to create a buffer or an image, allocate memory for it and bind them together, all in one call, you can use function vmaCreateBuffer(), vmaCreateImage(). This is the easiest and recommended way to use this library

vkcmdupdatebuffer - update small non-mappable memory region?
https://stackoverflow.com/questions/54673223/a-rarely-mentioned-vulkan-function-vkcmdupdatebuffer-what-is-it-used-for

Similar small reference implementation:
- https://github.com/EmbarkStudios/kajiya/blob/main/crates/lib/kajiya-backend/src/vulkan/buffer.rs
- https://github.com/adrien-ben/gltf-viewer-rs/blob/12054a20e39ba12c8b30c46bc83da55a8021f695/crates/libs/vulkan/src/buffer.rs
*/

pub struct VkBuffer {
  /// For debugging. User-set name
  name: String,
  /// For debugging. Includes size etc.
  long_name: String,
  /// Size in bytes
  pub size: usize,
  /// Native Vulkan buffer
  pub buffer: vk::Buffer,
  pub allocation: vma::Allocation,
  // mapping
  mapped_pointer: Option<MemoryMapPointer>,
}

// TODO [LOW] providing allocator to every fn is tiresome

impl VkBuffer {
  /// Allocate empty vulkan buffer
  pub fn empty(
    allocator: &vma::Allocator,
    queue_family: u32,
    name: String,
    size: usize,
    usage: vk::BufferUsageFlags,
    memory_pref: VkMemoryPreference,
  ) -> Self {
    let queue_family_indices = [queue_family];
    let buffer_info = vk::BufferCreateInfo::builder()
      .size(size as u64)
      .usage(usage)
      .sharing_mode(vk::SharingMode::EXCLUSIVE)
      .queue_family_indices(&queue_family_indices);

    let long_name = get_buffer_long_name(&name, size);
    let alloc_create_info = determine_gpu_allocation_info(&memory_pref);
    let (buffer, allocation) = unsafe {
      allocator
        .create_buffer(&buffer_info, &alloc_create_info)
        .expect(&format!("Failed allocating: {}", long_name))
    };
    let mapped_pointer = get_persistently_mapped_pointer(allocator, &allocation);

    Self {
      name: name.clone(),
      long_name,
      size,
      buffer,
      allocation,
      mapped_pointer,
    }
  }

  /// Allocate vulkan buffer and fill it with data
  pub fn from_data(
    allocator: &vma::Allocator,
    queue_family: u32,
    with_setup_cb: &impl WithSetupCmdBuffer,
    name: String,
    bytes: &[u8],
    usage: vk::BufferUsageFlags,
  ) -> Self {
    let size = bytes.len();

    // create CPU-mapped scratch buffer and later trasfer data to the final GPU-only buffer.
    // It's a performance optimization
    let mut scratch_buffer = VkBuffer::empty(
      allocator,
      queue_family,
      format!("{}-scratch-buffer", name),
      size,
      vk::BufferUsageFlags::TRANSFER_SRC,
      VkMemoryPreference::ScratchTransfer,
    );
    scratch_buffer.write_to_mapped(bytes);

    // create final buffer and transfer the content
    let buffer = VkBuffer::empty(
      allocator,
      queue_family,
      name,
      size,
      usage | vk::BufferUsageFlags::TRANSFER_DST,
      VkMemoryPreference::GpuOnly,
    );
    with_setup_cb.with_setup_cb(|device, cb| unsafe {
      let mem_region = ash::vk::BufferCopy::builder()
        .dst_offset(0)
        .src_offset(0)
        .size(size as u64)
        .build();
      device.cmd_copy_buffer(cb, scratch_buffer.buffer, buffer.buffer, &[mem_region]);
    });

    // cleanup tmp buffer
    unsafe { scratch_buffer.delete(allocator) };
    buffer
  }

  pub unsafe fn delete(&mut self, allocator: &vma::Allocator) -> () {
    allocator.destroy_buffer(self.buffer, &mut self.allocation)
  }
}

impl VkMemoryResource for VkBuffer {
  fn get_name(&self) -> &String {
    &self.name
  }

  fn get_long_name(&self) -> &String {
    &self.long_name
  }

  fn get_mapped_pointer(&self) -> Option<MemoryMapPointer> {
    self.mapped_pointer.clone()
  }
}

fn get_buffer_long_name(name: &String, size: usize) -> String {
  format!("VkBuffer '{}' ({} bytes)", name, size)
}
