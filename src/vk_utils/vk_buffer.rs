use ash::vk;

// https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/quick_start.html
// https://github.com/expenses/vulkan-base/blob/main/ash-helpers/src/lib.rs

// https://gpuopen-librariesandsdks.github.io/VulkanMemoryAllocator/html/choosing_memory_type.html
// If you want to create a buffer or an image, allocate memory for it and bind them together, all in one call, you can use function vmaCreateBuffer(), vmaCreateImage(). This is the easiest and recommended way to use this library

pub struct VkBuffer {
  pub buffer: vk::Buffer,
  pub allocation: vk_mem::Allocation,
}

impl VkBuffer {
  #[allow(dead_code)]
  pub fn empty(
    size: u64,
    usage: vk::BufferUsageFlags, // e.g. VK_BUFFER_USAGE_VERTEX_BUFFER_BIT
    allocator: &vk_mem::Allocator,
    queue_family: u32,
  ) -> Result<Self, u32> {
    let queue_family_indices = [queue_family];
    let buffer_info = vk::BufferCreateInfo::builder()
      .size(size)
      .usage(usage)
      .queue_family_indices(&queue_family_indices);
    let alloc_info = vk_mem::AllocationCreateInfo {
      usage: vk_mem::MemoryUsage::GpuOnly,
      ..Default::default()
    };
    let (buffer, allocation, _allocation_info) = allocator
      .create_buffer(&buffer_info, &alloc_info)
      .expect(&format!(
        "Failed allocating {:?} buffer of size {}",
        usage, size
      ));

    Ok(Self { buffer, allocation })
  }

  pub fn from_data(
    bytes: &[u8],
    usage: vk::BufferUsageFlags, // e.g. VK_BUFFER_USAGE_VERTEX_BUFFER_BIT
    allocator: &vk_mem::Allocator,
    queue_family: u32,
  ) -> Self {
    // TODO use Buffer::empty() to allocate
    let queue_family_indices = [queue_family];
    let buffer_info = vk::BufferCreateInfo::builder()
      .size(bytes.len() as u64)
      .usage(usage)
      .queue_family_indices(&queue_family_indices);

    // TODO create temp buffer with `vk::BufferUsageFlags::TRANSFER_SRC`, like in
    // `kajiya-main\crates\lib\kajiya-backend\src\vulkan\buffer.rs`:134
    let alloc_info = vk_mem::AllocationCreateInfo {
      usage: vk_mem::MemoryUsage::GpuOnly,
      required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE // TODO slow?
        | vk::MemoryPropertyFlags::HOST_COHERENT,
      ..Default::default()
    };
    let (buffer, allocation, _allocation_info) = allocator
      .create_buffer(&buffer_info, &alloc_info)
      .expect(&format!(
        "Failed allocating {:?} buffer of size {}",
        usage,
        bytes.len()
      ));

    // map buffer and copy content
    let pointer = allocator.map_memory(&allocation).expect(&format!(
      "Failed mapping {:?} buffer of size {} during Buffer::from_data",
      usage,
      bytes.len(),
    ));
    let slice = unsafe { std::slice::from_raw_parts_mut(pointer, bytes.len()) };
    slice.copy_from_slice(bytes);
    allocator.unmap_memory(&allocation).expect(&format!(
      "Failed unmapping {:?} buffer of size {} during Buffer::from_data",
      usage,
      bytes.len(),
    ));

    Self { buffer, allocation }
  }

  pub fn delete(&self, allocator: &vk_mem::Allocator) -> () {
    allocator
      .destroy_buffer(self.buffer, &self.allocation)
      .expect("Failed deleting buffer");
  }
}
