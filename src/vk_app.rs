use log::info;

use ash;
use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, Swapchain};
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use crate::vk_utils::buffer::AppVkBuffer;
use crate::vk_utils::resources::{create_fences, create_semaphores};

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

/**
https://www.khronos.org/assets/uploads/developers/library/2016-vulkan-devday-uk/7-Keeping-your-GPU-fed.pdf
*/
pub struct AppVkSynchronize {
  // one per each swapchain image:
  pub present_complete_semaphore: Vec<vk::Semaphore>,
  pub rendering_complete_semaphore: Vec<vk::Semaphore>,
  pub draw_commands_fences: Vec<vk::Fence>,
}

impl AppVkSynchronize {
  pub fn new(device: &ash::Device, frames_in_flight: usize) -> Self {
    Self {
      present_complete_semaphore: create_semaphores(device, frames_in_flight),
      rendering_complete_semaphore: create_semaphores(device, frames_in_flight),
      draw_commands_fences: create_fences(device, frames_in_flight),
    }
  }

  unsafe fn destroy(&self, device: &ash::Device) {
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
  // one per each swapchain image:
  pub cmd_buffers: Vec<vk::CommandBuffer>,
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

pub struct AppVkBuffers {
  pub triangle_vertex_buffer: AppVkBuffer,
}

impl AppVkBuffers {
  unsafe fn destroy(&self, allocator: &vk_mem::Allocator) {
    self.triangle_vertex_buffer.delete(allocator);
  }
}

/** Data per each frame-in-flight */
pub struct AppVkPerSwapchainImageData {
  pub command_buffer: vk::CommandBuffer,

  // synchronize
  pub present_complete_semaphore: vk::Semaphore,
  pub rendering_complete_semaphore: vk::Semaphore,
  pub draw_command_fence: vk::Fence,
}

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
pub struct AppVk {
  pub entry: ash::Entry,
  pub instance: ash::Instance,
  pub swapchain: AppVkSwapchain,
  pub synchronize: AppVkSynchronize,
  pub device: AppVkDevice,
  pub command_buffers: AppVkCommandBuffers,
  pub pipelines: AppVkPipelines,
  pub render_passes: AppVkRenderPasses,
  pub buffers: AppVkBuffers,
  pub allocator: vk_mem::Allocator,

  // surface
  pub surface_loader: Surface,
  pub surface_khr: vk::SurfaceKHR,

  // debug
  pub debug_utils_loader: DebugUtils,
  pub debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl AppVk {
  pub fn frames_in_flight(&self) -> usize {
    self.swapchain.image_views.len()
  }

  pub fn framebuffer_for_swapchain_image(&self, swapchain_image_index: u32) -> vk::Framebuffer {
    self.swapchain.framebuffers[swapchain_image_index as usize]
  }

  pub fn data_per_frame(&self, frame_idx: usize) -> AppVkPerSwapchainImageData {
    let cmd_bufs = &self.command_buffers.cmd_buffers;
    let syncs = &self.synchronize;

    AppVkPerSwapchainImageData {
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

  pub fn destroy(&mut self) {
    info!("AppVk::destroy()");
    let device = &self.device.device;
    unsafe {
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
      self.buffers.destroy(&self.allocator);
      self.allocator.destroy();

      self
        .debug_utils_loader
        .destroy_debug_utils_messenger(self.debug_messenger, None);

      // allocator.destroy();
      self.device.destroy();

      self.instance.destroy_instance(None);
    }
    info!("AppVk::destroy() finished");
  }
}
