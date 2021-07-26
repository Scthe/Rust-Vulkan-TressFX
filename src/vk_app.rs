use log::info;

use ash;
use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, Swapchain};
pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use crate::vk_utils::resources::{create_fence, create_semaphore};

///
/// This file contains kitchen sink for Vulkan stuff
///

pub struct AppVkSwapchain {
  pub swapchain_loader: Swapchain,
  pub swapchain: vk::SwapchainKHR,
  pub size: vk::Extent2D,

  // All fields below will be capabilites.min_images + 1
  pub framebuffers: Vec<vk::Framebuffer>,
  pub image_views: Vec<vk::ImageView>,
  pub images: Vec<vk::Image>,
}

impl AppVkSwapchain {
  unsafe fn destroy(&self, device: &ash::Device) {
    for &framebuffer in &self.framebuffers {
      device.destroy_framebuffer(framebuffer, None);
    }

    for &image_view in &self.image_views {
      device.destroy_image_view(image_view, None);
    }

    // Will also destroy images. From validation layers:
    // VK_OBJECT_TYPE_IMAGE; is a presentable image and it is controlled by the implementation and is destroyed with vkDestroySwapchainKHR.
    self
      .swapchain_loader
      .destroy_swapchain(self.swapchain, None);
  }
}

pub struct AppVkSynchronize {
  pub present_complete_semaphore: vk::Semaphore,
  pub rendering_complete_semaphore: vk::Semaphore,
  pub draw_commands_fence: vk::Fence,
}

impl AppVkSynchronize {
  pub fn new(device: &ash::Device) -> Self {
    Self {
      present_complete_semaphore: create_semaphore(device),
      rendering_complete_semaphore: create_semaphore(device),
      draw_commands_fence: create_fence(device),
    }
  }

  unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_semaphore(self.present_complete_semaphore, None);
    device.destroy_semaphore(self.rendering_complete_semaphore, None);
    device.destroy_fence(self.draw_commands_fence, None)
  }
}

pub struct AppVkDevice {
  pub phys_device: vk::PhysicalDevice,
  pub queue_family_index: u32,
  pub device: ash::Device,
  pub queue: vk::Queue,
}

impl AppVkDevice {
  unsafe fn destroy(&self) {
    self.device.destroy_device(None);
  }
}

pub struct AppVkCommandBuffers {
  pub pool: vk::CommandPool,
  // app specific:
  pub cmd_buf_triangle: vk::CommandBuffer,
}

impl AppVkCommandBuffers {
  unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_command_pool(self.pool, None);
  }
}

pub struct AppVkRenderPasses {
  // app specific:
  pub render_pass_triangle: vk::RenderPass,
}

impl AppVkRenderPasses {
  unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass_triangle, None);
  }
}

pub struct AppVkPipelines {
  pub pipeline_cache: vk::PipelineCache,
  // app specific:
  pub pipeline_triangle: vk::Pipeline,
  pub pipeline_triangle_layout: vk::PipelineLayout,
}

impl AppVkPipelines {
  unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_pipeline_layout(self.pipeline_triangle_layout, None);
    device.destroy_pipeline(self.pipeline_triangle, None);
    device.destroy_pipeline_cache(self.pipeline_cache, None);
  }
}

/** Kitchen sink for Vulkan stuff */
pub struct AppVk {
  pub entry: ash::Entry,
  pub instance: ash::Instance,
  pub swapchain: AppVkSwapchain,
  pub synchronize: AppVkSynchronize,
  pub device: AppVkDevice,
  pub command_buffers: AppVkCommandBuffers,
  pub pipelines: AppVkPipelines,
  pub render_passes: AppVkRenderPasses,

  // surface
  pub surface_loader: Surface,
  pub surface_khr: vk::SurfaceKHR,

  // debug
  pub debug_utils_loader: DebugUtils,
  pub debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl AppVk {
  pub unsafe fn destroy(&self) {
    info!("AppVk::destroy()");
    let device = &self.device.device;
    device.device_wait_idle().unwrap();

    self.synchronize.destroy(device);
    // depth_buffer.destroy(&device, &allocator).unwrap();
    // vertex_buffer.destroy(&allocator).unwrap();
    // index_buffer.destroy(&allocator).unwrap();
    self.command_buffers.destroy(device);
    self.swapchain.destroy(device);
    self.pipelines.destroy(device);
    self.render_passes.destroy(device);
    self.surface_loader.destroy_surface(self.surface_khr, None);

    self
      .debug_utils_loader
      .destroy_debug_utils_messenger(self.debug_messenger, None);

    // allocator.destroy();
    self.device.destroy();

    self.instance.destroy_instance(None);
    info!("AppVk::destroy() finished");
  }
}
