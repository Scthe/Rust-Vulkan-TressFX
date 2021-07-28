use ash::version::DeviceV1_0;
use ash::vk;

// https://github.com/zeux/niagara/blob/master/src/resources.cpp

pub fn create_image_view(
  device: &ash::Device,
  image: vk::Image,
  image_format: vk::Format,
  aspect_mask_flags: vk::ImageAspectFlags,
  base_mip_level: u32,
  mip_level_count: u32,
) -> vk::ImageView {
  let subresource_range = vk::ImageSubresourceRange::builder()
    .aspect_mask(aspect_mask_flags)
    .base_array_layer(0)
    .layer_count(1)
    .base_mip_level(base_mip_level)
    .level_count(mip_level_count)
    .build();

  let create_info = vk::ImageViewCreateInfo::builder()
    .image(image)
    .view_type(vk::ImageViewType::TYPE_2D)
    .format(image_format)
    .subresource_range(subresource_range)
    .build();

  unsafe {
    device
      .create_image_view(&create_info, None)
      .expect("Failed creating image view for swapchain image")
  }
}

pub fn create_semaphore(device: &ash::Device) -> vk::Semaphore {
  let semaphore_create_info = vk::SemaphoreCreateInfo::builder()
    .flags(vk::SemaphoreCreateFlags::empty())
    .build();
  unsafe {
    device
      .create_semaphore(&semaphore_create_info, None)
      .expect("Failed to create semaphore")
  }
}

pub fn create_fences(device: &ash::Device, count: usize) -> Vec<vk::Fence> {
  let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
  let mut result = Vec::<vk::Fence>::with_capacity(count);

  unsafe {
    for _ in 0..count {
      let fence = device
        .create_fence(&create_info, None)
        .expect("Failed to create fence");
      result.push(fence);
    }
  }

  result
}

pub fn create_viewport(size: &vk::Extent2D) -> vk::Viewport {
  vk::Viewport {
    x: 0f32,
    y: size.height as f32, // flip vulkan coord system - important!
    width: size.width as f32,
    height: -(size.height as f32), // flip vulkan coord system - important!
    min_depth: 0f32,
    max_depth: 1.0f32,
    ..Default::default()
  }
}

pub fn create_command_pool(device: &ash::Device, queue_family_index: u32) -> vk::CommandPool {
  // vk::CommandPoolCreateFlags::TRANSIENT - we are not short lived at all
  let cmd_pool_create_info = vk::CommandPoolCreateInfo::builder()
    .queue_family_index(queue_family_index)
    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
    .build();

  unsafe {
    let cmd_pool = device
      .create_command_pool(&cmd_pool_create_info, None)
      .expect("Failed creating command pool");

    // device
    // .reset_command_pool(cmd_pool, vk::CommandPoolResetFlags::empty())
    // .expect("Failed reseting command pool for 1st time");

    cmd_pool
  }
}

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
