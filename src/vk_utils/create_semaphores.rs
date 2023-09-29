use ash::version::DeviceV1_0;
use ash::vk;

pub fn create_semaphores(device: &ash::Device, count: usize) -> Vec<vk::Semaphore> {
  let semaphore_create_info = vk::SemaphoreCreateInfo::builder()
    .flags(vk::SemaphoreCreateFlags::empty())
    .build();
  let mut result = Vec::<vk::Semaphore>::with_capacity(count);

  for _ in 0..count {
    let obj = unsafe {
      device
        .create_semaphore(&semaphore_create_info, None)
        .expect("Failed to create semaphore")
    };
    result.push(obj);
  }

  result
}
