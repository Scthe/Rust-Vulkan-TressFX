use log::info;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;
use ash::vk;
use ash::{self, extensions::khr::PushDescriptor};

use super::*;
use crate::render_graph::FrameData;
use crate::vk_utils::{execute_setup_cmd_buf, WithSetupCmdBuffer};

/** Kitchen sink for Vulkan stuff */
pub struct VkCtx {
  // do not move these fields around, order matters for `Drop` trait
  pub allocator: vma::Allocator,
  pub device: VkCtxDevice,
  pub instance: ash::Instance,
  pub entry: ash::Entry,
  // rest of fields
  pub swapchain: VkCtxSwapchain,
  pub swapchain_images: Vec<VkCtxSwapchainImage>,
  pub command_pool: vk::CommandPool,
  // Special command buffer used for resource init
  pub setup_cb: vk::CommandBuffer,
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
  pub fn swapchain_images_count(&self) -> usize {
    self.swapchain_images.len()
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

  /// get next swapchain image
  /// https://themaister.net/blog/2023/11/12/my-scuffed-game-streaming-adventure-pyrofling/
  pub fn acquire_next_swapchain_image(&self, frame_data: &FrameData) -> &VkCtxSwapchainImage {
    let swapchain_image_index: usize = unsafe {
      // We *should* check the result for `VK_ERROR_OUT_OF_DATE_KHR`.
      // Recreate swapchain if that happens (usually after window resize/minimize).
      // Current code works on my PC so..
      self
        .swapchain
        .swapchain_loader
        .acquire_next_image(
          self.swapchain.swapchain,
          u64::MAX,
          frame_data.acquire_semaphore,
          vk::Fence::null(),
        )
        .expect("Failed to acquire next swapchain image")
        .0 as _
    };

    &self.swapchain_images[swapchain_image_index]
  }

  pub unsafe fn destroy(&mut self) {
    let device = &self.device.device;

    for obj in &self.swapchain_images {
      obj.destroy(device);
    }
    device.destroy_command_pool(self.command_pool, None);
    self.swapchain.destroy();
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
    let cmd_buf = self.setup_cb;
    unsafe { execute_setup_cmd_buf(device, queue, cmd_buf, callback) };
  }
}
