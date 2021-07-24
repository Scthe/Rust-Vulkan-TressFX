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

// ---------------

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

// ---------------

/// Create vulkan instance used to initialize practically everything else
#[cfg(all(windows))]
fn create_instance(entry: &ash::Entry) -> ash::Instance {
  let app_name = to_c_str(env!("CARGO_PKG_NAME"));
  let app_info = vk::ApplicationInfo {
    api_version: vk::make_version(1, 1, 0), // Vulkan 1.1.0, TODO update Nvidia driver and use 1.2.X
    application_version: get_app_version(),
    p_application_name: app_name,
    ..Default::default()
  };

  let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()]; // TODO not in prod
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

  trace!("Ash instance created");
  instance
}

fn setup_debug_reporting(entry: &ash::Entry, instance: &ash::Instance) -> () {
  let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
    .message_severity(
      vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
      // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE // will cause spam about extensions
    )
    .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
    .pfn_user_callback(Some(vulkan_debug_callback));

  let debug_utils_loader = DebugUtils::new(entry, instance);
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
) -> (vk::SwapchainKHR, vk::Extent2D, vk::Format) {
  let window_size = unsafe { get_window_size(window) };
  trace!("window_size {:?}", window_size);

  // surface_format. Only B8G8R8A8_UNORM, SRGB_NONLINEAR supported
  let surface_formats = unsafe {
    surface_meta
      .get_physical_device_surface_formats(*p_device, *surface_khr)
      .unwrap()
  };
  for &x in &surface_formats {
    trace!("Surface fmt: {:?}", x);
  }
  let only_one_i_personally_know = surface_formats.iter().find(|surface_fmt| {
    let fmt_ok = surface_fmt.format == vk::Format::B8G8R8A8_UNORM;
    let color_space_ok = surface_fmt.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR;
    fmt_ok && color_space_ok
  });
  let surface_format = match only_one_i_personally_know {
    Some(x) => x,
    _ => panic!("Failed to find swapchain surface format."),
  };

  let surface_capabilities = unsafe {
    surface_meta
      .get_physical_device_surface_capabilities(*p_device, *surface_khr)
      .unwrap()
  };
  trace!("surface_capabilities {:?}", surface_capabilities);

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
    .image_format(surface_format.format) // TODO change in swapchain_image_views too
    .image_color_space(surface_format.color_space)
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

  trace!("Swapchain created");
  (swapchain, window_size, surface_format.format)
}

fn create_swapchain_images(
  swapchain_meta: &Swapchain,
  swapchain: &vk::SwapchainKHR,
  device: &ash::Device,
  image_format: vk::Format,
) -> (Vec<vk::Image>, Vec<vk::ImageView>) {
  let swapchain_images = unsafe {
    swapchain_meta // auto destroyed with swapchain
      .get_swapchain_images(*swapchain)
      .expect("Failed to get swapchain images from swapchain")
  };
  trace!("Will create {} swapchain images", swapchain_images.len());
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
        .format(image_format)
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

  trace!("Swapchain images created");
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
    trace!("Found {} physical devices", phys_devices.len());

    // list of devices that satisfy our conditions
    let mut result = phys_devices.iter().filter_map(|&p_device| {
      let props = instance.get_physical_device_properties(p_device);
      let is_discrete = props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;
      // trace!("Physical device{:?}", props);

      let q_props = instance.get_physical_device_queue_family_properties(p_device);
      let mut graphic_fam_q_idx = q_props.iter().enumerate().filter_map(|(index, &q)| {
        trace!("Physical device :: queueFamily {:?}", props);
        let is_gfx = q.queue_flags.contains(vk::QueueFlags::GRAPHICS)
          && q.queue_flags.contains(vk::QueueFlags::COMPUTE)
          && q.queue_flags.contains(vk::QueueFlags::TRANSFER);
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
      None => panic!("No devices for Vulkan 1.1 found"),
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
      .expect("Failed to create (logical) device")
  };

  let queue = unsafe { device.get_device_queue(queue_family_index, 0) }; // only one queue created above
  trace!("Logical device selected");

  (device, queue)
}

fn create_render_pass(device: &ash::Device, image_format: vk::Format) -> vk::RenderPass {
  // 1. define render pass to compile shader against
  let attachment = vk::AttachmentDescription::builder()
    .format(image_format)
    .samples(vk::SampleCountFlags::TYPE_1) // single sampled
    .load_op(vk::AttachmentLoadOp::LOAD) // do not clear triangle background
    .store_op(vk::AttachmentStoreOp::STORE)
    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
    .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    .build();

  let subpass_output_attachment = vk::AttachmentReference {
    attachment: 0, // from the array above
    layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
  };
  let subpass = vk::SubpassDescription::builder()
    // .flags(flags) // No values in vk?
    .input_attachments(&[]) // INPUT: layout(input_attachment_index=X, set=Y, binding=Z)
    .color_attachments(&[subpass_output_attachment]) // OUTPUT
    // .depth_stencil_attachment(depth_stencil_attachment)
    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS) //
    // .preserve_attachments(preserve_attachments)
    // .resolve_attachments(resolve_attachments)
    .build();
  trace!("Subpass created, will be used to create render pass");

  let create_info = vk::RenderPassCreateInfo::builder()
    // .flags(vk::RenderPassCreateFlags::) // some BS about rotation 90dgr?
    // .dependencies() // ?
    // .pCorrelatedViewMasks() // ?
    .attachments(&[attachment])
    .subpasses(&[subpass])
    .build();
  let render_pass = unsafe {
    device
      .create_render_pass(&create_info, None)
      .expect("Failed creating render pass")
  };

  render_pass
}

fn create_framebuffer(
  device: &ash::Device,
  render_pass: &vk::RenderPass,
  image_view: &vk::ImageView,
  size: &vk::Extent2D,
) -> vk::Framebuffer {
  trace!("Will create framebuffer {}x{}", size.width, size.height);
  let create_info = vk::FramebufferCreateInfo::builder()
    .render_pass(*render_pass)
    .attachments(&[*image_view])
    .width(size.width)
    .height(size.height)
    .layers(1)
    .build();
  let framebuffer = unsafe {
    device
      .create_framebuffer(&create_info, None)
      .expect("Failed to create framebuffer")
  };
  trace!("Framebuffer created");

  framebuffer
}

fn load_shader(device: &ash::Device, path: &std::path::Path) -> vk::ShaderModule {
  trace!("Loading shader from {}", path.to_string_lossy());
  let content = std::fs::read(path).expect(&format!(
    "Failed opening shader file '{}'",
    path.to_string_lossy()
  ));
  // reinterpret ([u8,u8,u8,u8][u8,u8,u8,u8]...) => (u32, u32, ...)
  // do not use map, as this fills with 0s the rest of the byte

  // vk::ShaderModuleCreateInfo::builder().code(content).build(); // TODO try
  let create_info = vk::ShaderModuleCreateInfo {
    p_code: content.as_ptr() as *const u32,
    code_size: content.len(),
    ..Default::default()
  };
  let shader_module = unsafe {
    device
      .create_shader_module(&create_info, None)
      .expect(&format!(
        "Failed to create shader module from file '{}'",
        path.to_string_lossy()
      ))
  };

  trace!("Shader created OK (from {})", path.to_string_lossy());
  shader_module
}

fn create_pipeline(
  device: &ash::Device,
  shader_vs: &vk::ShaderModule,
  shader_fs: &vk::ShaderModule,
  render_pass: &vk::RenderPass,
) -> vk::Pipeline {
  trace!("Will create pipeline for a device, render pass based on shaders");
  let create_info = vk::PipelineCacheCreateInfo::builder().build();
  let pipeline_cache = unsafe {
    device
      .create_pipeline_cache(&create_info, None)
      .expect("Failed to create pipeline cache")
  };

  // create shaders from respective shader modules
  // TODO move to load_shader
  let shader_fn_name = unsafe { std::ffi::CStr::from_ptr("main".as_ptr() as *const i8) };
  let stage_vs = vk::PipelineShaderStageCreateInfo::builder()
    .stage(vk::ShaderStageFlags::VERTEX)
    .module(*shader_vs)
    .name(shader_fn_name)
    .build();
  let stage_fs = vk::PipelineShaderStageCreateInfo::builder()
    .stage(vk::ShaderStageFlags::FRAGMENT)
    .module(*shader_fs)
    .name(shader_fn_name)
    .build();
  // TODO dispose of shader modules?

  let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
    // TODO vertex desc here!
    .build();
  let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
    .build();
  // let viewport_state = vk::PipelineViewportStateCreateInfo::builder() // TODO dynamic, not baked in!
  // .viewports()
  // .scissors()
  // .build();
  // TODO dynamic, not baked in!
  let viewport_state = vk::PipelineViewportStateCreateInfo {
    viewport_count: 1,
    scissor_count: 1,
    ..Default::default()
  };
  let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
    .depth_clamp_enable(false) // when would You ever want it to be true?
    // .rasterizer_discard_enable(rasterizer_discard_enable)
    .polygon_mode(vk::PolygonMode::FILL)
    .cull_mode(vk::CullModeFlags::NONE) // for now
    .front_face(vk::FrontFace::CLOCKWISE) // TODO I don't remember OpenGL
    // .depth_bias_...
    .line_width(1.0) // validation layers: has to be 1.0 if not dynamic
    .build();
  let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
    .rasterization_samples(vk::SampleCountFlags::TYPE_1)
    .sample_shading_enable(false) // fragment shader per sample? Yes, please do! Oh wait, validation layers..
    // other sample coverage stuff
    // other alpha to coverage stuff
    .build();
  let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
    .depth_test_enable(false)
    .depth_write_enable(false)
    .depth_compare_op(vk::CompareOp::LESS) // IIRC?
    .depth_bounds_test_enable(false) // additional artificial depth test - has other variables here too
    .stencil_test_enable(false)
    .front(vk::StencilOpState {
      // compare_op etc..
      ..Default::default()
    })
    .back(vk::StencilOpState {
      // compare_op etc..
      ..Default::default()
    })
    .build();
  let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
    // I always hated blend state
    .attachments(&[vk::PipelineColorBlendAttachmentState{
      color_write_mask: vk::ColorComponentFlags::A | vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B,
      ..Default::default()
    }])
    // .blend_constants(blend_constants)
    .build();
  // We will provide during runtime
  let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
    .dynamic_states(&[
      vk::DynamicState::VIEWPORT,
      vk::DynamicState::SCISSOR,
      // other: depth, stencil, blend etc.
    ])
    .build();

  // TODO move from here?
  let create_info = vk::PipelineLayoutCreateInfo::builder()
    // texture/buffer bindings
    .build();
  let layout = unsafe {
    device
      .create_pipeline_layout(&create_info, None)
      .expect("Failed to create pipeline layout")
  };

  let create_info = vk::GraphicsPipelineCreateInfo::builder()
    // .flags(vk::PipelineCreateFlags::)
    .stages(&[stage_vs, stage_fs])
    .vertex_input_state(&vertex_input_state)
    .input_assembly_state(&input_assembly_state)
    // .tessellation_state(tessellation_state)
    .viewport_state(&viewport_state)
    .rasterization_state(&rasterization_state)
    .multisample_state(&multisample_state)
    .depth_stencil_state(&depth_stencil_state)
    .color_blend_state(&color_blend_state)
    .dynamic_state(&dynamic_state)
    .layout(layout)
    .render_pass(*render_pass)
    // .subpass()
    // .base_pipeline_handle(base_pipeline_handle)
    // .base_pipeline_index(base_pipeline_index)
    .build();
  unsafe {
    let pipelines = device
      .create_graphics_pipelines(pipeline_cache, &[create_info], None)
      .ok();
    match pipelines {
      Some(ps) if ps.len() > 0 => *ps.first().unwrap(),
      _ => panic!("Failed to create graphic pipeline"),
    }
  }
}

fn cmd_draw_triangle(
  device: &ash::Device,
  command_buffer: &vk::CommandBuffer,
  image_view: &vk::ImageView,
  size: &vk::Extent2D,
  image_format: vk::Format,
) -> () {
  // TODO move create_ code from cmd_draw_triangle, only draw_commands here
  let render_pass = create_render_pass(device, image_format);
  // TODO create this as array, just like image_views
  let framebuffer = create_framebuffer(device, &render_pass, image_view, size);
  let triangle_shader_vs = load_shader(&device, std::path::Path::new("./vert.spv"));
  let triangle_shader_fs = load_shader(&device, std::path::Path::new("./frag.spv"));

  let triangle_pipeline = create_pipeline(
    device,
    &triangle_shader_vs,
    &triangle_shader_fs,
    &render_pass,
  );

  let render_area = vk::Rect2D {
    offset: vk::Offset2D { x: 0, y: 0 },
    extent: *size,
  };
  let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
  .render_pass(render_pass)
  .framebuffer(framebuffer)
  .render_area(render_area)
  // .clear_values(clear_values) // clear color if needed
  .build();

  trace!("Registering commands to draw triangle");
  unsafe {
    /*
    device.cmd_pipeline_barrier(
      *command_buffer,
      vk::PipelineStageFlags::ALL_GRAPHICS,
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      dependency_flags,
      memory_barriers,
      buffer_memory_barriers,
      image_memory_barriers
    );*/

    device.cmd_begin_render_pass(
      *command_buffer,
      &render_pass_begin_info,
      vk::SubpassContents::INLINE,
    );

    // draw calls go here
    device.cmd_set_viewport(
      *command_buffer,
      0,
      &[vk::Viewport {
        x: 0f32,
        y: size.height as f32, // flip vulkan coord system - important!
        width: size.width as f32,
        height: -(size.height as f32), // flip vulkan coord system - important!
        min_depth: 0f32,
        max_depth: 1.0f32,
        ..Default::default()
      }],
    );
    device.cmd_set_scissor(*command_buffer, 0, &[render_area]);
    device.cmd_bind_pipeline(
      *command_buffer,
      vk::PipelineBindPoint::GRAPHICS,
      triangle_pipeline,
    );
    device.cmd_draw(*command_buffer, 3, 1, 0, 0);

    device.cmd_end_render_pass(*command_buffer)
  }
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
  let (swapchain, window_size, image_format) = create_swapchain_khr(
    &instance,
    &surface_meta,
    &surface_khr,
    &phys_device,
    &device,
    queue_family_index,
    window,
  );
  let (swapchain_images, swapchain_image_views) =
    create_swapchain_images(&swapchain_meta, &swapchain, &device, image_format);

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

  // TODO depth buffer: https://github.com/MaikKlein/ash/blob/master/examples/src/lib.rs#L490

  //
  // PER FRAME STARTS HERE
  let acquire_semaphore = create_semaphore(&device);
  let (swapchain_image_index, _) = swapchain_meta
    .acquire_next_image(swapchain, u64::MAX, acquire_semaphore, vk::Fence::null())
    .expect("Failed to acquire next swapchain image");
  let swapchain_image = swapchain_images[swapchain_image_index as usize];
  let swapchain_image_view = swapchain_image_views[swapchain_image_index as usize];

  //
  // start record command buffer
  let cmd_buf_begin_info = vk::CommandBufferBeginInfo::builder().build(); // can be one time submit bit for optimization
  device
    .begin_command_buffer(*cmd_buf, &cmd_buf_begin_info)
    .expect("Failed - begin_command_buffer");
  let clear_color = vk::ClearColorValue {
    float32: [0.2f32, 0.2f32, 0.2f32, 1f32],
  };
  // TODO not needed if we use clear color during pass begin
  let clear_image_range = vk::ImageSubresourceRange::builder()
    .aspect_mask(vk::ImageAspectFlags::COLOR)
    .base_array_layer(0)
    .layer_count(1) // array layers
    .base_mip_level(0)
    .level_count(1) // mip levels
    .build();
  device.cmd_clear_color_image(
    *cmd_buf,
    swapchain_image,
    vk::ImageLayout::GENERAL,
    &clear_color,
    &[clear_image_range],
  );

  cmd_draw_triangle(
    &device,
    cmd_buf,
    &swapchain_image_view,
    &window_size,
    image_format,
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
  device
    .device_wait_idle()
    .expect("Failed - device_wait_idle");

  Ok(())
}
