use ash::version::DeviceV1_0;
use ash::vk;

pub fn create_fences(device: &ash::Device, count: usize) -> Vec<vk::Fence> {
  let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
  let mut result = Vec::<vk::Fence>::with_capacity(count);

  for _ in 0..count {
    let obj = unsafe {
      device
        .create_fence(&create_info, None)
        .expect("Failed to create fence")
    };
    result.push(obj);
  }

  result
}
