use ash;
use ash::vk;

use crate::vk_utils::{create_fences, create_semaphores};

/**
https://www.khronos.org/assets/uploads/developers/library/2016-vulkan-devday-uk/7-Keeping-your-GPU-fed.pdf
*/
pub struct VkCtxSynchronize {
  // one per each swapchain image:
  pub present_complete_semaphore: Vec<vk::Semaphore>,
  pub rendering_complete_semaphore: Vec<vk::Semaphore>,
  pub draw_commands_fences: Vec<vk::Fence>,
}

impl VkCtxSynchronize {
  pub fn new(device: &ash::Device, frames_in_flight: usize) -> Self {
    Self {
      present_complete_semaphore: create_semaphores(device, frames_in_flight),
      rendering_complete_semaphore: create_semaphores(device, frames_in_flight),
      draw_commands_fences: create_fences(device, frames_in_flight),
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    for obj in &self.present_complete_semaphore {
      device.destroy_semaphore(*obj, None)
    }

    for obj in &self.rendering_complete_semaphore {
      device.destroy_semaphore(*obj, None)
    }

    for obj in &self.draw_commands_fences {
      device.destroy_fence(*obj, None)
    }
  }
}
