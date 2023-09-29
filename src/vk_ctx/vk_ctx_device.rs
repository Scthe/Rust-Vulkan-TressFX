use ash;
use ash::version::DeviceV1_0;
use ash::vk;

pub struct VkAppDevice {
  pub phys_device: vk::PhysicalDevice,
  pub queue_family_index: u32,
  pub device: ash::Device,
  pub queue: vk::Queue,
}

impl VkAppDevice {
  pub unsafe fn destroy(&self) {
    self.device.destroy_device(None);
  }
}
