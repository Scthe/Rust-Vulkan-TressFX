use log::{debug, error, info, warn};
use std::ffi::{CStr, CString};

use ash::extensions::ext::DebugUtils;
use ash::vk::{self, Handle, ObjectType};

/*
Recommended silenced messages (https://www.reddit.com/r/vulkan/comments/k5zqpp/did_you_know_muting_vulkan_validation_layer/):

(PERFORMANCE, "UNASSIGNED-BestPractices-vkImage-AvoidImageToImageCopy")
Only used during init, so no runtime cost

(PERFORMANCE, "UNASSIGNED-BestPractices-CreatePipelinesLayout-KeepLayoutSmall")
[AMD] debug layer. Nope!

(PERFORMANCE, "UNASSIGNED-BestPractices-CreateImage-TilingLinear")
[AMD] debug layer. Linear only used when we copy data to VK_IMAGE_TILING_OPTIMAL

(PERFORMANCE, "BestPractices-AllocateMemory-SetPriority")
[NVIDIA] debug layer. We use https://github.com/GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator
to not have to bother with this. TODO [LOW] `AllocationCreateInfo.priority: f32`?

(PERFORMANCE, "UNASSIGNED-BestPractices-CreateImage-Depth32Format")
[NVIDIA] TODO [LOW] We could just swap `vk::Format::D32_SFLOAT` to `vk::Format::D16_UNORM`


(PERFORMANCE, "UNASSIGNED-BestPractices-vkBindMemory-small-dedicated-allocation")
(PERFORMANCE, "UNASSIGNED-BestPractices-vkAllocateMemory-small-allocation")
TODO [LOW] Investigate spliting 1 big allocation for better memory alloc.
VulkanMemoryAllocator does not do this already??

(PERFORMANCE, "UNASSIGNED-BestPractices-ClearAttachment-ClearImage")
[AMD] TressFX PPLL heads image is cleared with `vkCmdClearColorImage`. Not a bug.

*/

/// called on validation layer message
/// https://github.com/EmbarkStudios/kajiya/blob/main/crates/lib/kajiya-backend/src/vulkan/instance.rs#L130
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
  graphics_debugging: bool,
) -> Option<(DebugUtils, vk::DebugUtilsMessengerEXT)> {
  if !graphics_debugging {
    return None;
  }

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

  Some((debug_utils_loader, debug_messenger))
}

////////////
// Debug names
// - https://renderdoc.org/docs/how/how_annotate_capture.html
// - https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_EXT_debug_utils.html

/// Used with RenderDoc to have readable names instead of generic 'Pass with 1 depth target'.
/// Works for both compute and graphic passess.
pub unsafe fn add_pass_debug_label(
  debug_utils: &DebugUtils,
  command_buffer: vk::CommandBuffer,
  name: &str,
) {
  let name_c = CString::new(name).unwrap();
  let marker = vk::DebugUtilsLabelEXT::builder()
    .label_name(&name_c)
    .build();
  debug_utils.cmd_begin_debug_utils_label(command_buffer, &marker);
}

unsafe fn set_object_debug_label(
  debug_utils: &DebugUtils,
  device: &vk::Device,
  object_type: ObjectType,
  object_handle: u64,
  name: &str,
) {
  let name_c = CString::new(name).unwrap();
  let name_info = vk::DebugUtilsObjectNameInfoEXT::builder()
    .object_type(object_type)
    .object_handle(object_handle)
    .object_name(&name_c)
    .build();
  debug_utils
    .set_debug_utils_object_name(*device, &name_info)
    .expect(&format!("Could not set name '{}'", name));
}

/// Add name in RenderDoc
pub unsafe fn set_texture_debug_label(
  debug_utils: &DebugUtils,
  device: &vk::Device,
  object_handle: vk::Image,
  name: &str,
) {
  set_object_debug_label(
    debug_utils,
    device,
    ObjectType::IMAGE,
    object_handle.as_raw(),
    name,
  );
}

/// Add name in RenderDoc
pub unsafe fn set_buffer_debug_label(
  debug_utils: &DebugUtils,
  device: &vk::Device,
  object_handle: vk::Buffer,
  name: &str,
) {
  set_object_debug_label(
    debug_utils,
    device,
    ObjectType::BUFFER,
    object_handle.as_raw(),
    name,
  );
}
