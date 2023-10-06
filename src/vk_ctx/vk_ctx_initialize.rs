use ash::version::DeviceV1_0;
use log::{info, trace};
use vk_mem;

use ash;
use ash::extensions::khr::{Surface, Swapchain};
use ash::vk;

use crate::vk_ctx::vk_ctx::VkCtx;
use crate::vk_ctx::vk_ctx_command_buffers::VkCtxCommandBuffers;
use crate::vk_ctx::vk_ctx_device::VkCtxDevice;
use crate::vk_ctx::vk_ctx_swapchain::VkCtxSwapchain;
use crate::vk_ctx::vk_ctx_synchronize::VkCtxSynchronize;

use crate::vk_utils::debug::setup_debug_reporting;
use crate::vk_utils::*;

#[cfg(all(windows))]
fn get_window_size(window: &winit::window::Window) -> vk::Extent2D {
  use winapi::shared::windef::HWND;
  use winapi::shared::windef::RECT;
  use winapi::um::winuser::GetClientRect;
  use winit::platform::windows::WindowExtWindows;

  let mut rect: RECT = RECT {
    left: 0,
    right: 0,
    bottom: 0,
    top: 0,
  };
  let hwnd = window.hwnd() as HWND;
  if unsafe { !GetClientRect(hwnd, &mut rect) } == 0 {
    panic!("Failed to get window size");
  }

  let w = rect.right - rect.left;
  let h = rect.bottom - rect.top;
  vk::Extent2D::builder()
    .width(w as u32)
    .height(h as u32)
    .build()
}

/// Glorified constructor for `VkCtx`, moved to separate file to be a bit cleaner.
///
/// Reference:
/// - https://github.com/MaikKlein/ash/blob/master/examples/src/lib.rs#L332
pub fn vk_ctx_initialize(window: &winit::window::Window) -> VkCtx {
  let (entry, instance) = create_instance();
  let (debug_utils_loader, debug_messenger) = setup_debug_reporting(&entry, &instance);

  // surface data
  let surface_loader = Surface::new(&entry, &instance); // I guess some generic OS-independent thing?
  let surface_khr = create_surface_khr(&entry, &instance, window); // real OS-backed thing

  // devices
  let (phys_device, queue_family_index) =
    pick_physical_device_and_queue_family_idx(&instance, &surface_loader, surface_khr);
  let (device, queue) = pick_device_and_queue(&instance, phys_device, queue_family_index);

  // swapchain - prepare
  let window_size = get_window_size(window);
  trace!("window_size {:?}", window_size);
  let swapchain_format = get_swapchain_format(&surface_loader, surface_khr, phys_device)
    .expect("Could not find valid surface format");
  let surface_capabilities = get_surface_capabilities(phys_device, &surface_loader, surface_khr);

  // swapchain
  let swapchain_loader = Swapchain::new(&instance, &device); // I guess some generic OS-independent thing?
  let swapchain = create_swapchain_khr(
    &swapchain_loader,
    surface_khr,
    &swapchain_format,
    surface_capabilities,
    &window_size,
    queue_family_index,
  );
  let (swapchain_images, swapchain_image_views) = create_swapchain_images(
    &swapchain_loader,
    swapchain,
    &device,
    swapchain_format.format,
  );
  let frames_in_flight = swapchain_images.len() as u32;
  info!("Will use {} frames in flight", frames_in_flight);

  // command buffers
  let cmd_pool = create_command_pool(&device, queue_family_index);
  let cmd_bufs = create_command_buffers(&device, cmd_pool, frames_in_flight);

  // gpu memory allocator
  let allocator = vk_mem::Allocator::new(&vk_mem::AllocatorCreateInfo {
    // All 3 needed by VMA
    device: device.clone(),
    physical_device: phys_device,
    instance: instance.clone(),
    // cfg:
    flags: vk_mem::AllocatorCreateFlags::NONE, // or maybe even EXTERNALLY_SYNCHRONIZED ?
    preferred_large_heap_block_size: 0,        // pls only <256 MiB
    frame_in_use_count: frames_in_flight,
    heap_size_limits: None, // ugh, I guess?
  })
  .expect("Failed creating memory allocator (VMA lib init)");

  // pipeline_cache
  let pipeline_cache = create_pipeline_cache(&device);

  // descriptor_pool
  // TODO util that takes `(&[vk::DescriptorType], frames_in_flight)`?
  let descriptor_pool_size = vk::DescriptorPoolSize::builder()
    .ty(vk::DescriptorType::UNIFORM_BUFFER)
    .descriptor_count(frames_in_flight)
    .build();
  let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
    .pool_sizes(&[descriptor_pool_size])
    .max_sets(frames_in_flight)
    .build();
  let descriptor_pool = unsafe {
    device
      .create_descriptor_pool(&descriptor_pool_create_info, None)
      .expect("Failed reseting command pool for 1st time")
  };

  VkCtx {
    entry,
    instance,
    allocator,
    swapchain: VkCtxSwapchain {
      swapchain_loader,
      swapchain,
      size: window_size,
      surface_format: swapchain_format,
      image_views: swapchain_image_views,
      images: swapchain_images,
    },
    synchronize: VkCtxSynchronize::new(&device, frames_in_flight as usize),
    device: VkCtxDevice {
      phys_device,
      queue_family_index,
      device,
      queue,
    },
    command_buffers: VkCtxCommandBuffers {
      pool: cmd_pool,
      cmd_buffers: cmd_bufs,
    },
    pipeline_cache,
    descriptor_pool,
    surface_loader,
    surface_khr,
    debug_utils_loader,
    debug_messenger,
  }
}
