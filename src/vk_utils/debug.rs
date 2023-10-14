use log::{debug, error, info, warn};
use std::borrow::Cow;
use std::ffi::CStr;

use ash::extensions::ext::DebugUtils;
use ash::vk;

/// Added after https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_push_descriptor.html.
/// Seems validation layer does not work properly with this extension? It reports descriptor set as unbound,
/// while we are sure that it IS bound (and shader also has access to it!).
/// This would be removed with https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_EXT_descriptor_indexing.html
const MSG_A: &str = " The Vulkan spec states: For each set n that is statically used by a bound shader, a descriptor set must have been bound to n at the same pipeline bind point, with a VkPipelineLayout that is compatible for set n, with the VkPipelineLayout or VkDescriptorSetLayout array that was used to create the current VkPipeline or VkShaderEXT, as described in Pipeline Layout Compatibility";

fn is_message_ignored(message: &Cow<'_, str>) -> bool {
  message.contains(MSG_A)
}

// called on validation layer message
extern "system" fn vulkan_debug_callback(
  message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
  message_type: vk::DebugUtilsMessageTypeFlagsEXT,
  p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
  _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
  let callback_data = unsafe { *p_callback_data };
  // let message_id_number: i32 = callback_data.message_id_number as i32;
  // let message_id_name = CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy();
  let message = unsafe { CStr::from_ptr(callback_data.p_message).to_string_lossy() };

  if is_message_ignored(&message) {
    return vk::FALSE;
  }

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
    .message_type(
      vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
    )
    .pfn_user_callback(Some(vulkan_debug_callback))
    .build();

  let debug_utils_loader = DebugUtils::new(entry, instance);

  let debug_messenger = unsafe {
    debug_utils_loader
      .create_debug_utils_messenger(&debug_info, None)
      .unwrap()
  };

  (debug_utils_loader, debug_messenger)
}
