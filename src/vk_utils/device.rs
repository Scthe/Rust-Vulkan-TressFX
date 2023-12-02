use log::{info, trace};
use std::ffi::{CStr, CString};

use ash::extensions::{
  ext::DebugUtils,
  khr::{PushDescriptor, Surface, Swapchain},
};
use ash::vk;

#[cfg(target_os = "windows")]
use ash::extensions::khr::Win32Surface;

fn from_c_str<'a>(s: &[std::os::raw::c_char]) -> &'a CStr {
  unsafe { std::ffi::CStr::from_ptr(s.as_ptr() as *const std::os::raw::c_char) }
}

fn get_app_version() -> u32 {
  let to_u32 = |s: &str| s.parse::<u32>().unwrap();

  ash::vk::api_version_major(1);
  ash::vk::make_api_version(
    0,
    to_u32(env!("CARGO_PKG_VERSION_MAJOR")),
    to_u32(env!("CARGO_PKG_VERSION_MINOR")),
    to_u32(env!("CARGO_PKG_VERSION_PATCH")),
  )
}

fn get_layer_names(graphics_debugging: bool) -> Vec<CString> {
  let mut layer_names = Vec::new();
  if graphics_debugging {
    layer_names.push(CString::new("VK_LAYER_KHRONOS_validation").unwrap());
  }
  layer_names
}

fn get_extension_names(graphics_debugging: bool) -> Vec<*const i8> {
  let mut names = vec![Surface::name().as_ptr(), Win32Surface::name().as_ptr()];
  if graphics_debugging {
    names.push(DebugUtils::name().as_ptr());
  }
  names
}

#[cfg(all(windows))]
pub fn create_instance(graphics_debugging: bool) -> (ash::Entry, ash::Instance) {
  let entry = unsafe { ash::Entry::load().expect("Failed to create ash::Entry") };

  let app_name = CString::new(env!("CARGO_PKG_NAME")).unwrap();

  let app_info = vk::ApplicationInfo::builder()
    .application_name(&app_name)
    .application_version(get_app_version())
    .api_version(vk::make_api_version(0, 1, 3, 0))
    .build();

  // https://github.com/EmbarkStudios/kajiya/blob/main/crates/lib/kajiya-backend/src/vulkan/instance.rs#L52
  let layer_names = get_layer_names(graphics_debugging);
  let layers_names_raw: Vec<*const i8> = layer_names
    .iter()
    .map(|raw_name| raw_name.as_ptr())
    .collect();

  let extension_names = get_extension_names(graphics_debugging);
  let extension_names_raw: Vec<*const i8> = extension_names.iter().copied().collect();

  let create_info = vk::InstanceCreateInfo::builder()
    .application_info(&app_info)
    .enabled_layer_names(&layers_names_raw)
    .enabled_extension_names(&extension_names_raw)
    .build();

  let instance: ash::Instance = unsafe {
    entry
      .create_instance(&create_info, None)
      .expect("Failed to create ash::Instance")
  };

  trace!("Ash instance created");
  (entry, instance)
}

fn find_queue_family(
  instance: &ash::Instance,
  surface_loader: &Surface,
  surface_khr: vk::SurfaceKHR,
  phys_device: vk::PhysicalDevice,
) -> Option<usize> {
  let q_props = unsafe { instance.get_physical_device_queue_family_properties(phys_device) };

  let mut graphic_fam_q_idx = q_props.iter().enumerate().filter_map(|(index, &q)| {
    // trace!("Physical device :: queueFamily {:?}", q_props);
    let is_gfx = q.queue_flags.contains(vk::QueueFlags::GRAPHICS)
      && q.queue_flags.contains(vk::QueueFlags::COMPUTE)
      && q.queue_flags.contains(vk::QueueFlags::TRANSFER);

    let is_present_support = unsafe {
      surface_loader
        .get_physical_device_surface_support(phys_device, index as u32, surface_khr)
        .expect("Failed checking if physical device can present on our surface")
    };

    if is_gfx && is_present_support {
      Some(index)
    } else {
      None
    }
  });

  graphic_fam_q_idx.next()
}

/// Picks physical device e.g. "GeForce GTX 1050 Ti" and graphic queue family index.
/// Same physical device will also be used to present result
pub fn pick_physical_device_and_queue_family_idx(
  instance: &ash::Instance,
  surface_loader: &Surface,
  surface_khr: vk::SurfaceKHR,
) -> (vk::PhysicalDevice, u32) {
  let phys_devices = unsafe {
    instance
      .enumerate_physical_devices()
      .expect("Failed to enumerate physical devices")
  };
  trace!("Found {} physical devices", phys_devices.len());

  // list of devices that satisfy our conditions
  let mut result = phys_devices.iter().filter_map(|&phys_device| {
    let props = unsafe { instance.get_physical_device_properties(phys_device) };
    let features = unsafe { instance.get_physical_device_features(phys_device) };
    // trace!("Physical device{:?}", props);

    let is_discrete = props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;
    let has_anisotropy = features.sampler_anisotropy != vk::FALSE;
    let has_in_fragment_store = features.fragment_stores_and_atomics != vk::FALSE;
    let phys_device_ok = is_discrete && has_anisotropy && has_in_fragment_store;

    let graphic_fam_q_idx = find_queue_family(instance, surface_loader, surface_khr, phys_device);
    match graphic_fam_q_idx {
      Some(idx) if phys_device_ok => Some((phys_device, idx)),
      _ => None,
    }
  });

  match result.next() {
    None => panic!("No devices for Vulkan 1.3 found"),
    Some((p_device, idx)) => {
      let props = unsafe { instance.get_physical_device_properties(p_device) };
      let device_name = from_c_str(&props.device_name);
      info!("Using physical device: {:?}", device_name);
      (p_device, idx as u32)
    }
  }
}

/// Pick logical device
pub fn pick_device_and_queue(
  instance: &ash::Instance,
  phys_device: vk::PhysicalDevice,
  queue_family_index: u32,
) -> (ash::Device, vk::Queue) {
  trace!("Will pick logical device");
  let queue_prio = [1.0f32]; // only one queue
  let queue_create_infos = vk::DeviceQueueCreateInfo::builder()
    .queue_family_index(queue_family_index)
    .queue_priorities(&queue_prio)
    .build();

  // Arseny:
  // https://github.com/zeux/niagara/blob/master/src/device.cpp#L181
  // `VK_KHR_shader_non_semantic_info` was promoted to Vulkan 1.3 and is no longer needed!
  let device_extension_names_raw = [Swapchain::name().as_ptr(), PushDescriptor::name().as_ptr()];

  let mut separate_depth_stencil = vk::PhysicalDeviceSeparateDepthStencilLayoutsFeatures::builder()
    .separate_depth_stencil_layouts(true)
    .build();
  let mut device_features_13 = vk::PhysicalDeviceVulkan13Features::builder()
    .synchronization2(true)
    .build();
  let device_create_info = vk::DeviceCreateInfo::builder()
    .queue_create_infos(&[queue_create_infos])
    .enabled_extension_names(&device_extension_names_raw)
    .enabled_features(&vk::PhysicalDeviceFeatures {
      sampler_anisotropy: vk::TRUE,
      fragment_stores_and_atomics: vk::TRUE,
      independent_blend: vk::TRUE,
      ..Default::default()
    })
    .push_next(&mut separate_depth_stencil)
    .push_next(&mut device_features_13)
    .build();

  let device: ash::Device = unsafe {
    instance
      .create_device(phys_device, &device_create_info, None)
      .expect("Failed to create (logical) device")
  };
  trace!("Logical device selected");

  let queue = unsafe { device.get_device_queue(queue_family_index, 0) }; // only one queue created above
  trace!("Queue on logical device selected");

  (device, queue)
}
