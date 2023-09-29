use ash::version::DeviceV1_0;
use ash::vk;

pub fn create_command_buffers(
  device: &ash::Device,
  cmd_pool: vk::CommandPool,
  count: usize,
) -> Vec<vk::CommandBuffer> {
  let cmd_buf_create_info = vk::CommandBufferAllocateInfo::builder()
    .command_buffer_count(count as u32)
    .command_pool(cmd_pool)
    .level(vk::CommandBufferLevel::PRIMARY)
    .build();

  unsafe {
    device
      .allocate_command_buffers(&cmd_buf_create_info)
      .expect("Failed allocating command buffer")
  }
}
