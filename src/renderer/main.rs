use log::{debug, error, info, trace, warn};
use std::ffi::{CStr, CString};

use ash;
use ash::extensions::{
  ext::DebugUtils,
  khr::{Surface, Swapchain},
};
pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

#[cfg(target_os = "windows")]
use ash::extensions::khr::Win32Surface;

fn to_u32(s: &str) -> u32 {
  s.parse::<u32>().unwrap()
}

fn get_app_version() -> u32 {
  return vk::make_version(
    to_u32(env!("CARGO_PKG_VERSION_MAJOR")),
    to_u32(env!("CARGO_PKG_VERSION_MINOR")),
    to_u32(env!("CARGO_PKG_VERSION_PATCH")),
  );
}

fn to_c_str(s: &str) -> *const i8 {
  CString::new(s).unwrap().as_ptr()
}

fn from_c_str<'a>(s: &[std::os::raw::c_char]) -> &'a CStr {
  unsafe { std::ffi::CStr::from_ptr(s.as_ptr() as *const std::os::raw::c_char) }
}

#[cfg(all(windows))]
unsafe fn get_window_size(window: &winit::window::Window) -> vk::Extent2D {
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
  if !GetClientRect(hwnd, &mut rect) == 0 {
    panic!("Failed to get window size");
  }

  let w = rect.right - rect.left;
  let h = rect.bottom - rect.top;
  vk::Extent2D::builder()
    .width(w as u32)
    .height(h as u32)
    .build()
}

// ---------------

// called on validation layer message
unsafe extern "system" fn vulkan_debug_callback(
  message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
  message_type: vk::DebugUtilsMessageTypeFlagsEXT,
  p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
  _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
  let callback_data = *p_callback_data;
  let message_id_number: i32 = callback_data.message_id_number as i32;
  let message_id_name = CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy();
  let message = CStr::from_ptr(callback_data.p_message).to_string_lossy();

  let message_str = format!(
    "[VK, {:?}]: {}", // "[VK, {:?}]: [{} ({})] : {}\n",
    message_type,
    // message_id_name,
    // &message_id_number.to_string(),
    message,
  );

  match message_severity {
    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => error!("{}", message_str),
    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("{}", message_str),
    vk::DebugUtilsMessageSeverityFlagsEXT::INFO => info!("{}", message_str),
    _ => debug!("{}", message_str),
  }

  vk::FALSE
}

// ---------------

/// Create vulkan instance used to initialize practically everything else
#[cfg(all(windows))]
fn create_instance(entry: &ash::Entry) -> ash::Instance {
  let app_name = to_c_str(env!("CARGO_PKG_NAME"));
  let app_info = vk::ApplicationInfo {
    api_version: vk::make_version(1, 1, 0),
    application_version: get_app_version(),
    p_application_name: app_name,
    ..Default::default()
  };

  let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
  let layers_names_raw: Vec<*const i8> = layer_names
    .iter()
    .map(|raw_name| raw_name.as_ptr())
    .collect();
  let extension_names = vec![
    Surface::name().as_ptr(),
    Win32Surface::name().as_ptr(),
    DebugUtils::name().as_ptr(),
  ];
  let extension_names_raw: Vec<*const i8> = extension_names.iter().copied().collect();

  let create_info = vk::InstanceCreateInfo::builder()
    .application_info(&app_info)
    .enabled_layer_names(&layers_names_raw)
    .enabled_extension_names(&extension_names_raw)
    .build();

  let instance: ash::Instance = unsafe {
    entry
      .create_instance(&create_info, None)
      .expect("Failed to create instance!")
  };

  instance
}

fn setup_debug_reporting(entry: &ash::Entry, instance: &ash::Instance) -> () {
  let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
    .message_severity(
      // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
      // vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
      vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
    )
    .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
    .pfn_user_callback(Some(vulkan_debug_callback));

  let debug_utils_loader = DebugUtils::new(entry, instance);
  /*let debug_call_back =*/
  unsafe {
    debug_utils_loader
      .create_debug_utils_messenger(&debug_info, None)
      .unwrap();
  };
}

/// Gets surface from OS window
#[cfg(target_os = "windows")]
unsafe fn create_surface_khr(
  entry: &ash::Entry,
  instance: &ash::Instance,
  window: &winit::window::Window,
) -> vk::SurfaceKHR {
  use std::ptr;
  use winapi::shared::windef::HWND;
  use winapi::um::libloaderapi::GetModuleHandleW;
  use winit::platform::windows::WindowExtWindows;

  let hwnd = window.hwnd() as HWND;
  let hinstance = GetModuleHandleW(ptr::null()) as *const libc::c_void;
  let win32_create_info = vk::Win32SurfaceCreateInfoKHR {
    s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
    p_next: ptr::null(),
    flags: Default::default(),
    hinstance,
    hwnd: hwnd as *const libc::c_void,
  };
  let win32_surface_factory = Win32Surface::new(entry, instance);
  win32_surface_factory
    .create_win32_surface(&win32_create_info, None)
    .expect("Failed to create win32 surface")
}

/// Creates OS-dependent swapchain
fn create_swapchain_khr(
  instance: &ash::Instance,
  surface_meta: &Surface,
  surface_khr: &vk::SurfaceKHR,
  p_device: &vk::PhysicalDevice,
  device: &ash::Device,
  queue_familiy_idx: u32,
  window: &winit::window::Window,
) -> vk::SwapchainKHR {
  let window_size = unsafe { get_window_size(window) };
  // println!("{:?}", window_size);

  /*// surface_format
  let surface_formats = surface_meta
    .get_physical_device_surface_formats(*p_device, *surface_khr)
    .unwrap();
  surface_formats.iter().for_each(|f| println!("surface format: {:?}", f));
  let surface_format = surface_formats.first()
    .expect("Failed to find swapchain surface format.");
  */

  let surface_capabilities = unsafe {
    surface_meta
      .get_physical_device_surface_capabilities(*p_device, *surface_khr)
      .unwrap()
  };

  // pre_transform
  let can_identity = surface_capabilities
    .supported_transforms
    .contains(vk::SurfaceTransformFlagsKHR::IDENTITY);
  let pre_transform = if can_identity {
    vk::SurfaceTransformFlagsKHR::IDENTITY
  } else {
    surface_capabilities.current_transform
  };

  let create_info = vk::SwapchainCreateInfoKHR::builder()
    .surface(*surface_khr)
    .min_image_count(2) // double buffer
    .image_format(vk::Format::B8G8R8A8_UNORM) // not all devices support this e.g. BGRA only
    .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
    // .image_format(surface_format.format) // TODO change in swapchain_image_views too
    // .image_color_space(surface_format.color_space)
    .image_extent(window_size)
    .image_array_layers(1)
    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
    .queue_family_indices(&[queue_familiy_idx])
    .present_mode(vk::PresentModeKHR::FIFO) // guaranteed!
    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
    .pre_transform(pre_transform)
    .build();

  let swapchain_loader = Swapchain::new(instance, device);

  let swapchain = unsafe {
    swapchain_loader
      .create_swapchain(&create_info, None)
      .expect("Failed to create swapchain")
  };

  swapchain
}

fn create_swapchain_images(
  swapchain_meta: &Swapchain,
  swapchain: &vk::SwapchainKHR,
  device: &ash::Device,
) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
  let swapchain_images = unsafe {
    swapchain_meta // auto destroyed with swapchain
      .get_swapchain_images(*swapchain)
      .expect("Failed to get swapchain images from swapchain")
  };
  let swapchain_image_view_components_mapping = vk::ComponentMapping::builder()
    .r(vk::ComponentSwizzle::R)
    .g(vk::ComponentSwizzle::G)
    .b(vk::ComponentSwizzle::B)
    .a(vk::ComponentSwizzle::A)
    .build();
  let swapchain_image_view_subresource_range = vk::ImageSubresourceRange::builder()
    .aspect_mask(vk::ImageAspectFlags::COLOR)
    .base_array_layer(0)
    .layer_count(1)
    .base_mip_level(0)
    .level_count(1) // mip levels
    .build();
  let swapchain_image_views: Vec<vk::ImageView> = swapchain_images
    .iter()
    .map(|&swapch_image| {
      let create_info = vk::ImageViewCreateInfo::builder() // TODO destroy views manually
        .image(swapch_image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(vk::Format::B8G8R8A8_UNORM) // TODO: hardcoded, use from swapchain_create_info
        .components(swapchain_image_view_components_mapping)
        .subresource_range(swapchain_image_view_subresource_range)
        .build();

      unsafe {
        device
          .create_image_view(&create_info, None)
          .expect("Failed creating image view for swapchain image")
      }
    })
    .collect();

  (swapchain_images, swapchain_image_views)
}

fn create_semaphore(device: &ash::Device) -> vk::Semaphore {
  let semaphore_create_info = vk::SemaphoreCreateInfo::builder()
    .flags(vk::SemaphoreCreateFlags::empty())
    .build();
  unsafe {
    device
      .create_semaphore(&semaphore_create_info, None)
      .expect("Failed to create acquire semaphore")
  }
}

/// Picks physical device e.g. "GeForce GTX 1050 Ti" and graphic queue family index.
/// Same physical device will also be used to present result
fn pick_physical_device_queue_family_idx(
  instance: &ash::Instance,
  surface_meta: &Surface,
  surface_khr: &vk::SurfaceKHR,
) -> (vk::PhysicalDevice, u32) {
  unsafe {
    let phys_devices = instance
      .enumerate_physical_devices()
      .expect("Failed to enumerate physical devices");
    info!("Found compatible devices: {}", phys_devices.len());

    let mut result = phys_devices.iter().filter_map(|&p_device| {
      let props = instance.get_physical_device_properties(p_device);
      let is_discrete = props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;
      // debug!("{:?}", props);

      let q_props = instance.get_physical_device_queue_family_properties(p_device);
      let mut graphic_fam_q_idx = q_props.iter().enumerate().filter_map(|(index, &q)| {
        let is_gfx = q.queue_flags.contains(vk::QueueFlags::GRAPHICS);
        let is_present_support = surface_meta
          .get_physical_device_surface_support(p_device, index as u32, *surface_khr)
          .expect("Failed checking if physical device can present on our surface");

        if is_discrete && is_gfx && is_present_support {
          Some(index)
        } else {
          None
        }
      });

      graphic_fam_q_idx.next().map(|index| (p_device, index))
    });

    match result.next() {
      None => panic!("No devices for Vulkan 1.2 found"),
      Some((p_device, idx)) => {
        let props = instance.get_physical_device_properties(p_device);
        info!(
          "Using physical device: {:?}",
          from_c_str(&props.device_name)
        );
        (p_device, idx as u32)
      }
    }
  }
}

/// Pick logical device
fn pick_device_and_queue(
  instance: &ash::Instance,
  phys_device: &vk::PhysicalDevice,
  queue_family_index: u32,
) -> (ash::Device, vk::Queue) {
  let queue_prio = [1.0f32]; // only one queue
  let queue_create_infos = vk::DeviceQueueCreateInfo::builder()
    .queue_family_index(queue_family_index)
    .queue_priorities(&queue_prio)
    .build();

  let device_extension_names_raw = [Swapchain::name().as_ptr()];

  let device_create_info = vk::DeviceCreateInfo::builder()
    .queue_create_infos(&[queue_create_infos])
    .enabled_extension_names(&device_extension_names_raw)
    .build();

  let device: ash::Device = unsafe {
    instance
      .create_device(*phys_device, &device_create_info, None)
      .expect("Failed to create device")
  };

  let queue = unsafe { device.get_device_queue(queue_family_index, 0) }; // only one queue created above

  (device, queue)
}

// https://github.com/MaikKlein/ash/blob/master/examples/src/lib.rs#L332
pub unsafe fn main(window: &winit::window::Window) -> anyhow::Result<()> {
  let entry = ash::Entry::new().expect("Failed creating ash::Entry");
  let instance = create_instance(&entry);
  setup_debug_reporting(&entry, &instance);

  // surface data
  let surface_meta = Surface::new(&entry, &instance); // I guess some generic OS-independent thing?
  let surface_khr = create_surface_khr(&entry, &instance, window); // real OS-backed thing

  // devices
  let (phys_device, queue_family_index) =
    pick_physical_device_queue_family_idx(&instance, &surface_meta, &surface_khr);
  let (device, queue) = pick_device_and_queue(&instance, &phys_device, queue_family_index);

  // swapchain
  let swapchain_meta = Swapchain::new(&instance, &device); // I guess some generic OS-independent thing?
  let swapchain = create_swapchain_khr(
    &instance,
    &surface_meta,
    &surface_khr,
    &phys_device,
    &device,
    queue_family_index,
    window,
  );
  let (swapchain_images, swapchain_image_views) =
    create_swapchain_images(&swapchain_meta, &swapchain, &device);

  ///////////////////////////////////////
  let cmd_pool_create_info = vk::CommandPoolCreateInfo::builder()
    // .flags(vk::CommandPoolCreateFlags::TRANSIENT) // we are not short lived at all
    .queue_family_index(queue_family_index)
    .build();
  let cmd_pool = device
    .create_command_pool(&cmd_pool_create_info, None)
    .expect("Failed creating command pool");
  device
    .reset_command_pool(cmd_pool, vk::CommandPoolResetFlags::empty())
    .expect("Failed reseting command pool for 1st time");

  let cmd_buf_create_info = vk::CommandBufferAllocateInfo::builder()
    .command_buffer_count(1) // one command buffer
    .command_pool(cmd_pool)
    .level(vk::CommandBufferLevel::PRIMARY)
    .build();
  let cmd_bufs = device
    .allocate_command_buffers(&cmd_buf_create_info)
    .expect("Failed allocating command buffer");
  let cmd_buf = cmd_bufs
    .first()
    .expect("Failed - no command buffers were actually created?!");

  // TODO depth buffer: https://github.com/MaikKlein/ash/blob/master/examples/src/lib.rs#L548

  //
  // PER FRAME STARTS HERE
  let acquire_semaphore = create_semaphore(&device);
  let (swapchain_image_index, _) = swapchain_meta
    .acquire_next_image(swapchain, u64::MAX, acquire_semaphore, vk::Fence::null())
    .expect("Failed to acquire next swapchain image");

  //
  // start record command buffer
  let cmd_buf_begin_info = vk::CommandBufferBeginInfo::builder().build(); // can be one time submit bit for optimization
  device
    .begin_command_buffer(*cmd_buf, &cmd_buf_begin_info)
    .expect("Failed - begin_command_buffer");
  let clear_color = vk::ClearColorValue {
    float32: [0f32, 1f32, 0f32, 1f32],
  };
  let clear_image_range = vk::ImageSubresourceRange::builder()
    .aspect_mask(vk::ImageAspectFlags::COLOR)
    .base_array_layer(0)
    .layer_count(1) // array layers
    .base_mip_level(0)
    .level_count(1) // mip levels
    .build();
  device.cmd_clear_color_image(
    *cmd_buf,
    swapchain_images[swapchain_image_index as usize],
    vk::ImageLayout::GENERAL,
    &clear_color,
    &[clear_image_range],
  );

  device
    .end_command_buffer(*cmd_buf)
    .expect("Failed - end_command_buffer");
  // end record command buffer
  //

  // submit to the queue
  let release_semaphore = create_semaphore(&device);
  let dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
  let submit_info = vk::SubmitInfo::builder()
    .wait_semaphores(&[acquire_semaphore])
    .command_buffers(&[*cmd_buf])
    .wait_dst_stage_mask(&dst_stage_mask)
    .signal_semaphores(&[release_semaphore])
    .build();
  device
    .queue_submit(queue, &[submit_info], vk::Fence::null())
    .expect("Failed queue submit");

  // present queue result
  let swapchains = [swapchain];
  let image_indices = [swapchain_image_index];
  let present_info = vk::PresentInfoKHR::builder()
    .image_indices(&image_indices)
    // .results(results) // p_results: ptr::null_mut(),
    .swapchains(&swapchains)
    .wait_semaphores(&[release_semaphore])
    .build();
  unsafe {
    swapchain_meta
      .queue_present(queue, &present_info)
      .expect("Failed to execute queue present.");
  }
  device.device_wait_idle();

  Ok(())
}
