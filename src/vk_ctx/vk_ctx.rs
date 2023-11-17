use log::info;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;
use ash::vk;
use ash::{self, extensions::khr::PushDescriptor};

use super::*;
use crate::vk_utils::{execute_setup_cmd_buf, WithSetupCmdBuffer};

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

// TODO [CRITICAL] Check Vulkan validation layers: best practices, synchronization, portability, nsight-gpu-trace log

/** Kitchen sink for Vulkan stuff */
pub struct VkCtx {
  // do not move these fields around, order matters for `Drop` trait
  pub allocator: vma::Allocator,
  pub device: VkCtxDevice,
  pub instance: ash::Instance,
  pub entry: ash::Entry,
  // rest of fields
  pub swapchain: VkCtxSwapchain,
  pub synchronize: VkCtxSynchronize,
  pub command_buffers: VkCtxCommandBuffers,
  pub pipeline_cache: vk::PipelineCache,
  pub push_descriptor: PushDescriptor,
  /// C'mon you will not use non-linear/nearest sampling anyway, can just create global objects..
  pub default_texture_sampler_linear: vk::Sampler,
  pub default_texture_sampler_nearest: vk::Sampler,

  // surface
  pub surface_loader: Surface,
  pub surface_khr: vk::SurfaceKHR,

  // debug
  pub debug_utils: Option<(DebugUtils, vk::DebugUtilsMessengerEXT)>,
}

impl VkCtx {
  pub fn frames_in_flight(&self) -> usize {
    self.swapchain.image_views.len()
  }

  pub fn window_size(&self) -> vk::Extent2D {
    self.swapchain.size
  }

  pub fn vk_device(&self) -> &ash::Device {
    &self.device.device
  }

  pub fn with_debug_loader(&self, callback: impl FnOnce(&DebugUtils)) {
    self.debug_utils.as_ref().map(|dbg| callback(&dbg.0));
  }

  pub fn data_per_frame(&self, frame_idx: usize) -> VkCtxPerSwapchainImageData {
    let cmd_bufs = &self.command_buffers.cmd_buffers;
    let syncs = &self.synchronize;

    VkCtxPerSwapchainImageData {
      swapchain_image_idx: frame_idx,
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
    let device = &self.device.device;

    self.synchronize.destroy(device);
    self.command_buffers.destroy(device);
    self.swapchain.destroy(device);
    device.destroy_pipeline_cache(self.pipeline_cache, None);
    self.surface_loader.destroy_surface(self.surface_khr, None);
    device.destroy_sampler(self.default_texture_sampler_linear, None);
    device.destroy_sampler(self.default_texture_sampler_nearest, None);

    info!("VkCtx::destroy() finished. All app resources should be deleted. Only Device, Allocator and Instance remain.");
  }
}

impl Drop for VkCtx {
  fn drop(&mut self) {
    unsafe {
      match &self.debug_utils {
        Some((loader, messanger)) => {
          loader.destroy_debug_utils_messenger(*messanger, None);
        }
        _ => (),
      }

      // allocator and device are removed through `Drop` trait
      // self.instance.destroy_instance(None); // ?
    }
  }
}

impl WithSetupCmdBuffer for VkCtx {
  fn with_setup_cb(&self, callback: impl FnOnce(&ash::Device, vk::CommandBuffer)) {
    let device = &self.device.device;
    let queue = self.device.queue;
    let cmd_buf = self.command_buffers.setup_cb;
    unsafe { execute_setup_cmd_buf(device, queue, cmd_buf, callback) };
  }
}
