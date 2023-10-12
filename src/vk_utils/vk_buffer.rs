use ash::vk;
use std::marker::{Send, Sync};
use vma::Alloc;

// https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/quick_start.html
// https://github.com/expenses/vulkan-base/blob/main/ash-helpers/src/lib.rs

// https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/choosing_memory_type.html
// If you want to create a buffer or an image, allocate memory for it and bind them together, all in one call, you can use function vmaCreateBuffer(), vmaCreateImage(). This is the easiest and recommended way to use this library

/// Wrapper over a raw pointer to make it moveable and accessible from other threads
pub struct MemoryMapPointer(pub *mut u8);
unsafe impl Send for MemoryMapPointer {}
unsafe impl Sync for MemoryMapPointer {}

pub struct VkBuffer {
  // For debugging
  pub name: String,
  /// Size in bytes
  pub size: usize,
  /// Native Vulkan buffer
  pub buffer: vk::Buffer,
  pub allocation: vma::Allocation,
  // mapping
  mapped_pointer: Option<MemoryMapPointer>,
}

// TODO providing allocator to every fn is tiresome

fn fmt_buf_name(name: &String, size: usize) -> String {
  format!("Buffer '{}' ({} bytes)", name, size)
}

impl VkBuffer {
  /// Allocate empty vulkan buffer
  /// * `mappable` - optimize for CPU mapping to copy CPU->GPU data
  pub fn empty(
    name: String,
    size: usize,
    usage: vk::BufferUsageFlags,
    allocator: &vma::Allocator,
    queue_family: u32,
    mappable: bool,
  ) -> Self {
    let queue_family_indices = [queue_family];
    let buffer_info = vk::BufferCreateInfo::builder()
      .size(size as u64)
      .usage(usage)
      .sharing_mode(vk::SharingMode::EXCLUSIVE)
      .queue_family_indices(&queue_family_indices);

    #[allow(deprecated)]
    let mut alloc_info = vma::AllocationCreateInfo {
      usage: vma::MemoryUsage::GpuOnly,
      ..Default::default()
    };
    if mappable {
      alloc_info.required_flags =
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
    }

    let (buffer, allocation) = unsafe {
      allocator
        .create_buffer(&buffer_info, &alloc_info)
        .expect(&format!("Failed allocating: {}", fmt_buf_name(&name, size)))
    };

    Self {
      name,
      size,
      buffer,
      allocation,
      mapped_pointer: None,
    }
  }

  /// Allocate vulkan buffer and fill it with data
  pub fn from_data(
    name: String,
    bytes: &[u8],
    usage: vk::BufferUsageFlags,
    allocator: &vma::Allocator,
    queue_family: u32,
  ) -> Self {
    let size = bytes.len();
    let mut buffer = VkBuffer::empty(name, size, usage, allocator, queue_family, true);

    // TODO create temp buffer with `vk::BufferUsageFlags::TRANSFER_SRC`, like in
    // `kajiya-main\crates\lib\kajiya-backend\src\vulkan\buffer.rs`:134
    // Requires 'setup command pool' (.with_setup_cb(|cb| { ... })) for cmd_copy_buffer
    // https://vulkan-tutorial.com/Texture_mapping/Images#page_Layout-transitions - also used for images

    // map buffer and copy content
    buffer.map_memory(allocator);
    buffer.write_to_mapped(bytes);
    buffer.unmap_memory(allocator);

    buffer
  }

  pub fn map_memory(&mut self, allocator: &vma::Allocator) -> *mut u8 {
    if let Some(ptr) = &self.mapped_pointer {
      ptr.0
    } else {
      let pointer = unsafe {
        allocator
          .map_memory(&mut self.allocation)
          .expect(&format!("Failed mapping: {}", self.name()))
      };
      self.mapped_pointer = Some(MemoryMapPointer(pointer));
      pointer
    }
  }

  pub fn unmap_memory(&mut self, allocator: &vma::Allocator) {
    if self.mapped_pointer.take().is_some() {
      unsafe { allocator.unmap_memory(&mut self.allocation) };
    }
  }

  pub fn write_to_mapped(&self, bytes: &[u8]) {
    let size = bytes.len();

    if let Some(pointer) = &self.mapped_pointer {
      let slice = unsafe { std::slice::from_raw_parts_mut(pointer.0, size) };
      slice.copy_from_slice(bytes);
    } else {
      panic!("Tried to write {} bytes to unmapped {}", size, self.name())
    }
  }

  pub fn name(&self) -> String {
    fmt_buf_name(&self.name, self.size)
  }

  pub unsafe fn delete(&mut self, allocator: &vma::Allocator) -> () {
    allocator.destroy_buffer(self.buffer, &mut self.allocation)
  }
}
