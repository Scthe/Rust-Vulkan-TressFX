use log::{error, trace};

use ash::extensions::khr::{Surface, Swapchain};
use ash::vk;

#[cfg(target_os = "windows")]
use ash::extensions::khr::Win32Surface;

use crate::config::Config;
use crate::either;
use crate::vk_utils::create_image_view;

/*
// https://github.com/zeux/niagara/blob/master/src/swapchain.cpp#L78
struct Swapchain
{
  VkSwapchainKHR swapchain;

  std::vector<VkImage> images;

  uint32_t width, height;
  uint32_t imageCount;
};
*/
pub fn size_to_rect_vk(size: &vk::Extent2D) -> vk::Rect2D {
  vk::Rect2D {
    offset: vk::Offset2D { x: 0, y: 0 },
    extent: *size,
  }
}

/// Gets surface from OS window
#[cfg(target_os = "windows")]
pub fn create_surface_khr(
  entry: &ash::Entry,
  instance: &ash::Instance,
  window: &winit::window::Window,
) -> vk::SurfaceKHR {
  use std::ptr;
  use winapi::shared::windef::HWND;
  use winapi::um::libloaderapi::GetModuleHandleW;
  use winit::platform::windows::WindowExtWindows;

  let hwnd = window.hwnd() as HWND;
  let hinstance = unsafe { GetModuleHandleW(ptr::null()) as *const libc::c_void };
  let win32_create_info = vk::Win32SurfaceCreateInfoKHR::builder()
    .hinstance(hinstance)
    .hwnd(hwnd as *const libc::c_void)
    .build();

  let win32_surface_factory = Win32Surface::new(entry, instance);
  unsafe {
    win32_surface_factory
      .create_win32_surface(&win32_create_info, None)
      .expect("Failed to create win32 surface for khr::Win32Surface extension")
  }
}

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkSurfaceFormatKHR.html
pub fn get_swapchain_format(
  surface_loader: &Surface,
  surface_khr: vk::SurfaceKHR,
  phys_device: vk::PhysicalDevice,
) -> Option<vk::SurfaceFormatKHR> {
  // surface_format. Only B8G8R8A8_UNORM, SRGB_NONLINEAR supported
  let surface_formats = unsafe {
    surface_loader
      .get_physical_device_surface_formats(phys_device, surface_khr)
      .unwrap()
  };
  // for &x in &surface_formats {
  // trace!("Surface fmt: {:?}", x);
  // }

  // return surface_formats.first();

  // TBH there is only one that I know
  // https://stackoverflow.com/questions/66401081/vulkan-swapchain-format-unorm-vs-srgb
  let fmt = surface_formats.iter().find(|surface_fmt| {
    let fmt_ok = surface_fmt.format == vk::Format::B8G8R8A8_UNORM;
    let color_space_ok = surface_fmt.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR;
    fmt_ok && color_space_ok
  });

  fmt.map(|x| x.to_owned())
}

pub fn get_surface_capabilities(
  device: vk::PhysicalDevice,
  surface_loader: &Surface,
  surface_khr: vk::SurfaceKHR,
) -> vk::SurfaceCapabilitiesKHR {
  let surface_capabilities = unsafe {
    surface_loader
      .get_physical_device_surface_capabilities(device, surface_khr)
      .unwrap()
  };
  trace!("Surface_capabilities {:?}", surface_capabilities);
  surface_capabilities
}

fn get_pre_transform(
  surface_capabilities: vk::SurfaceCapabilitiesKHR,
) -> vk::SurfaceTransformFlagsKHR {
  // Check if surface supports SurfaceTransformFlagsKHR::IDENTITY
  let can_identity = surface_capabilities
    .supported_transforms
    .contains(vk::SurfaceTransformFlagsKHR::IDENTITY);
  if can_identity {
    vk::SurfaceTransformFlagsKHR::IDENTITY
  } else {
    surface_capabilities.current_transform
  }
}

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkPresentModeKHR.html
/// https://github.com/EmbarkStudios/kajiya/blob/main/crates/lib/kajiya-backend/src/vulkan/swapchain.rs#L85
pub fn get_present_mode(
  surface_loader: &Surface,
  surface_khr: vk::SurfaceKHR,
  phys_device: vk::PhysicalDevice,
  vsync: bool,
) -> vk::PresentModeKHR {
  let present_mode_preference = if vsync {
    vec![vk::PresentModeKHR::FIFO_RELAXED, vk::PresentModeKHR::FIFO]
  } else {
    vec![vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::IMMEDIATE]
  };

  let present_modes = unsafe {
    surface_loader
      .get_physical_device_surface_present_modes(phys_device, surface_khr)
      .expect("Failed to get surface present modes")
  };

  present_mode_preference
    .into_iter()
    .find(|mode| present_modes.contains(mode))
    .unwrap_or(vk::PresentModeKHR::FIFO) // FIFO is guaranteed
}

fn get_alpha_composite(
  surface_capabilites: &vk::SurfaceCapabilitiesKHR,
) -> vk::CompositeAlphaFlagsKHR {
  let mut composite_alpha = either!(
    Config::TEST_ALPHA_COMPOSITE,
    vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
    vk::CompositeAlphaFlagsKHR::OPAQUE
  );
  if !surface_capabilites
    .supported_composite_alpha
    .contains(composite_alpha)
  {
    error!(
      "Unsupported CompositeAlphaFlagsKHR. Wanted {:?}, but only {:?} are available",
      composite_alpha, surface_capabilites.supported_composite_alpha
    );
    composite_alpha = vk::CompositeAlphaFlagsKHR::OPAQUE;
  }

  composite_alpha
}

/// Creates OS-dependent swapchain
pub fn create_swapchain_khr(
  swapchain_loader: &Swapchain,
  surface_khr: vk::SurfaceKHR,
  surface_format: &vk::SurfaceFormatKHR,
  surface_capabilites: vk::SurfaceCapabilitiesKHR,
  size: &vk::Extent2D,
  queue_familiy_idx: u32,
  present_mode: vk::PresentModeKHR,
) -> vk::SwapchainKHR {
  let image_count = surface_capabilites
    .max_image_count
    .min(surface_capabilites.min_image_count + 1);

  let composite_alpha = get_alpha_composite(&surface_capabilites);
  let create_info = vk::SwapchainCreateInfoKHR::builder()
    .surface(surface_khr)
    .min_image_count(image_count)
    .image_format(surface_format.format)
    .image_color_space(surface_format.color_space)
    .image_extent(*size)
    .image_array_layers(1)
    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
    .queue_family_indices(&[queue_familiy_idx])
    .present_mode(present_mode)
    .composite_alpha(composite_alpha)
    .pre_transform(get_pre_transform(surface_capabilites))
    .clipped(true)
    .build();

  let swapchain = unsafe {
    swapchain_loader
      .create_swapchain(&create_info, None)
      .expect("Failed to create swapchain")
  };
  trace!("Swapchain created");
  swapchain
}

pub fn get_swapchain_images(
  swapchain_loader: &Swapchain,
  swapchain: vk::SwapchainKHR,
) -> Vec<vk::Image> {
  unsafe {
    swapchain_loader
      .get_swapchain_images(swapchain)
      .expect("Failed to get swapchain images from swapchain")
  }
}

pub fn create_swapchain_image_view(
  device: &ash::Device,
  swapchain_image: vk::Image,
  image_format: vk::Format,
) -> vk::ImageView {
  let aspect_mask_flags = vk::ImageAspectFlags::COLOR;
  create_image_view(device, swapchain_image, image_format, aspect_mask_flags)
}
