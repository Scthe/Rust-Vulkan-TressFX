use log::{info, trace};
use vma;

use ash;
use ash::extensions::khr::{PushDescriptor, Surface, Swapchain};
use ash::vk::{self};

use crate::vk_ctx::vk_ctx::VkCtx;
use crate::vk_ctx::vk_ctx_device::VkCtxDevice;
use crate::vk_ctx::vk_ctx_swapchain::VkCtxSwapchain;
use crate::vk_ctx::VkCtxSwapchainImage;

use crate::vk_utils::debug::setup_debug_reporting;
use crate::vk_utils::*;

// TODO [LOW] Move to src\vk_utils\os.rs
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
///
/// TODO [LOW] Move to src\vk_ctx\vk_ctx.rs
pub fn vk_ctx_initialize(
  window: &winit::window::Window,
  graphics_debugging: bool,
  vsync: bool,
) -> VkCtx {
  let (entry, instance) = create_instance(graphics_debugging);
  let debug_utils = setup_debug_reporting(&entry, &instance, graphics_debugging);

  // surface data
  let surface_loader = Surface::new(&entry, &instance); // I guess some generic OS-independent thing?
  let surface_khr = create_surface_khr(&entry, &instance, window); // real OS-backed thing

  // devices
  let (phys_device, queue_family_index) =
    pick_physical_device_and_queue_family_idx(&instance, &surface_loader, surface_khr);
  let (device, queue) = pick_device_and_queue(&instance, phys_device, queue_family_index);

  // push descriptor set as alternative to descriptor set pools etc.
  let push_descriptor = PushDescriptor::new(&instance, &device);

  // swapchain - prepare
  let window_size = get_window_size(window);
  trace!("window_size {:?}", window_size);
  let swapchain_format = get_swapchain_format(&surface_loader, surface_khr, phys_device)
    .expect("Could not find valid surface format");
  let surface_capabilities = get_surface_capabilities(phys_device, &surface_loader, surface_khr);

  // swapchain
  let swapchain_loader = Swapchain::new(&instance, &device); // I guess some generic OS-independent thing?
  let present_mode = get_present_mode(&surface_loader, surface_khr, phys_device, vsync);
  let swapchain = create_swapchain_khr(
    &swapchain_loader,
    surface_khr,
    &swapchain_format,
    surface_capabilities,
    &window_size,
    queue_family_index,
    present_mode,
  );
  let swapchain_images = get_swapchain_images(&swapchain_loader, swapchain);
  info!("Will use {} swapchain images", swapchain_images.len());

  let per_swapchain_image_data: Vec<VkCtxSwapchainImage> = swapchain_images
    .iter()
    .enumerate()
    .map(|(idx, image)| VkCtxSwapchainImage::new(&device, idx, *image, swapchain_format.format))
    .collect();

  // command buffers
  let command_pool = create_command_pool(&device, queue_family_index);

  // setup cmd buffer
  let setup_cb = create_command_buffer(&device, command_pool);

  // gpu memory allocator
  let allocator_create_info = vma::AllocatorCreateInfo::new(&instance, &device, phys_device);
  let allocator = vma::Allocator::new(allocator_create_info)
    .expect("Failed creating memory allocator (VMA lib init)");

  // pipeline_cache
  let pipeline_cache = create_pipeline_cache(&device);

  // sampler
  let sampler_linear = create_sampler(
    &device,
    vk::Filter::LINEAR,
    vk::Filter::LINEAR,
    vk::SamplerMipmapMode::LINEAR,
  );
  let sampler_nearest = create_sampler(
    &device,
    vk::Filter::NEAREST,
    vk::Filter::NEAREST,
    vk::SamplerMipmapMode::NEAREST,
  );

  VkCtx {
    entry,
    instance,
    allocator,
    command_pool,
    setup_cb,
    swapchain: VkCtxSwapchain {
      swapchain_loader,
      swapchain,
      size: window_size,
      surface_format: swapchain_format,
    },
    swapchain_images: per_swapchain_image_data,
    device: VkCtxDevice {
      phys_device,
      queue_family_index,
      device,
      queue,
    },
    pipeline_cache,
    push_descriptor,
    surface_loader,
    surface_khr,
    debug_utils,
    default_texture_sampler_linear: sampler_linear,
    default_texture_sampler_nearest: sampler_nearest,
  }
}
