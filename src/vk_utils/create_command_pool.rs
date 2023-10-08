use ash::vk;

pub fn create_command_pool(device: &ash::Device, queue_family_index: u32) -> vk::CommandPool {
  // vk::CommandPoolCreateFlags::TRANSIENT - we are not short lived at all
  let cmd_pool_create_info = vk::CommandPoolCreateInfo::builder()
    .queue_family_index(queue_family_index)
    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER | vk::CommandPoolCreateFlags::TRANSIENT)
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
