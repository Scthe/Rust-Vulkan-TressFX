use ash;
use ash::vk;

pub struct VkCtxDevice {
  pub phys_device: vk::PhysicalDevice,
  pub queue_family_index: u32,
  pub device: ash::Device,
  pub queue: vk::Queue,
}

impl Drop for VkCtxDevice {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_device(None);
    }
  }
}
