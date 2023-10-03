use log::info;

use ash;
use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use super::*;

fn get_resource_at_idx<T: std::marker::Copy>(res_name: &str, arr: &Vec<T>, idx: usize) -> T {
  let obj = arr.get(idx);
  assert!(
    obj.is_some(),
    "Requested {} for {}th in-flight frame, there are only {}",
    res_name,
    idx,
    arr.len()
  );

  obj.unwrap().clone()
}

/** Kitchen sink for Vulkan stuff */
pub struct VkCtx {
  pub entry: ash::Entry,
  pub instance: ash::Instance,
  pub swapchain: VkCtxSwapchain,
  pub synchronize: VkCtxSynchronize,
  pub device: VkCtxDevice,
  pub command_buffers: VkCtxCommandBuffers,
  pub pipeline_cache: vk::PipelineCache,
  pub allocator: vk_mem::Allocator,

  // surface
  pub surface_loader: Surface,
  pub surface_khr: vk::SurfaceKHR,

  // debug
  pub debug_utils_loader: DebugUtils,
  pub debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VkCtx {
  pub fn frames_in_flight(&self) -> usize {
    self.swapchain.image_views.len()
  }

  pub fn data_per_frame(&self, frame_idx: usize) -> VkCtxPerSwapchainImageData {
    let cmd_bufs = &self.command_buffers.cmd_buffers;
    let syncs = &self.synchronize;

    VkCtxPerSwapchainImageData {
      command_buffer: get_resource_at_idx("command_buffer", cmd_bufs, frame_idx),
      draw_command_fence: get_resource_at_idx("fence", &syncs.draw_commands_fences, frame_idx),
      present_complete_semaphore: get_resource_at_idx(
        "present_complete_semaphore",
        &syncs.present_complete_semaphore,
        frame_idx,
      ),
      rendering_complete_semaphore: get_resource_at_idx(
        "rendering_complete_semaphore",
        &syncs.rendering_complete_semaphore,
        frame_idx,
      ),
    }
  }

  pub unsafe fn destroy(&mut self) {
    info!("AppVk::destroy()");
    let device = &self.device.device;
    // device.device_wait_idle().unwrap();

    self.synchronize.destroy(device);
    // depth_buffer.destroy(&device, &allocator).unwrap();
    // vertex_buffer.destroy(&allocator).unwrap();
    // index_buffer.destroy(&allocator).unwrap();
    self.command_buffers.destroy(device);
    self.swapchain.destroy(device);
    device.destroy_pipeline_cache(self.pipeline_cache, None);
    self.surface_loader.destroy_surface(self.surface_khr, None);
    self.allocator.destroy();

    self
      .debug_utils_loader
      .destroy_debug_utils_messenger(self.debug_messenger, None);

    self.device.destroy();

    self.instance.destroy_instance(None);
    info!("AppVk::destroy() finished");
  }
}
