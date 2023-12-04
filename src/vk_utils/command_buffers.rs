use ash::vk;

pub fn create_command_pool(device: &ash::Device, queue_family_index: u32) -> vk::CommandPool {
  // vk::CommandPoolCreateFlags::TRANSIENT - we are not short lived at all
  let cmd_pool_create_info = vk::CommandPoolCreateInfo::builder()
    .queue_family_index(queue_family_index)
    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
    .build();

  let cmd_pool = unsafe {
    device
      .create_command_pool(&cmd_pool_create_info, None)
      .expect("Failed creating command pool")
  };

  unsafe {
    device
      .reset_command_pool(cmd_pool, vk::CommandPoolResetFlags::empty())
      .expect("Failed reseting command pool for 1st time");
  }

  cmd_pool
}

pub fn create_command_buffer(device: &ash::Device, cmd_pool: vk::CommandPool) -> vk::CommandBuffer {
  let cmd_buf_create_info = vk::CommandBufferAllocateInfo::builder()
    .command_buffer_count(1)
    .command_pool(cmd_pool)
    .level(vk::CommandBufferLevel::PRIMARY)
    .build();

  let cmd_buffers = unsafe {
    device
      .allocate_command_buffers(&cmd_buf_create_info)
      .expect("Failed allocating command buffer")
  };
  cmd_buffers[0]
}

// TODO [NOW] remove
pub fn create_command_buffers(
  device: &ash::Device,
  cmd_pool: vk::CommandPool,
  count: u32,
) -> Vec<vk::CommandBuffer> {
  let cmd_buf_create_info = vk::CommandBufferAllocateInfo::builder()
    .command_buffer_count(count)
    .command_pool(cmd_pool)
    .level(vk::CommandBufferLevel::PRIMARY)
    .build();
  unsafe {
    device
      .allocate_command_buffers(&cmd_buf_create_info)
      .expect("Failed allocating command buffer")
  }
}
