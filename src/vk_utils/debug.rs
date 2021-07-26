use log::{debug, error, info, warn};
use std::ffi::CStr;

use ash::extensions::ext::DebugUtils;
use ash::vk;

// called on validation layer message
unsafe extern "system" fn vulkan_debug_callback(
  message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
  message_type: vk::DebugUtilsMessageTypeFlagsEXT,
  p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
  _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
  let callback_data = *p_callback_data;
  // let message_id_number: i32 = callback_data.message_id_number as i32;
  // let message_id_name = CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy();
  let message = CStr::from_ptr(callback_data.p_message).to_string_lossy();

  let message_str = format!(
    "[VK_dbg_callback, {:?}]: {}", // "[VK, {:?}]: [{} ({})] : {}\n",
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

pub fn setup_debug_reporting(
  entry: &ash::Entry,
  instance: &ash::Instance,
) -> (DebugUtils, vk::DebugUtilsMessengerEXT) {
  let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
    .message_severity(
      vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
      // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE // will cause spam about extensions
    )
    .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
    .pfn_user_callback(Some(vulkan_debug_callback))
    .build();

  let debug_utils_loader = DebugUtils::new(entry, instance);
  unsafe {
    let debug_messenger = debug_utils_loader
      .create_debug_utils_messenger(&debug_info, None)
      .unwrap();

    (debug_utils_loader, debug_messenger)
  }
}
