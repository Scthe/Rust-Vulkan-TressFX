use log::{info, trace};

use ash;
use ash::extensions::khr::{Surface, Swapchain};
pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use crate::vk_app::{
  AppVk, AppVkCommandBuffers, AppVkDevice, AppVkPipelines, AppVkRenderPasses, AppVkSwapchain,
  AppVkSynchronize,
};
use crate::vk_utils::debug::setup_debug_reporting;
use crate::vk_utils::device::{
  create_instance, pick_device_and_queue, pick_physical_device_and_queue_family_idx,
};
use crate::vk_utils::fbo::create_framebuffer;
use crate::vk_utils::pipeline::{
  create_pipeline_cache, ps_color_write_all, ps_depth_always_stencil_always, ps_dynamic_state,
  ps_ia_triangle_list, ps_multisample_disabled, ps_raster_polygons, ps_viewport_single_dynamic,
};
use crate::vk_utils::resources::{create_command_buffers, create_command_pool};
use crate::vk_utils::shaders::load_shader;
use crate::vk_utils::swapchain::{
  create_surface_khr, create_swapchain_images, create_swapchain_khr, get_surface_capabilities,
  get_swapchain_format,
};

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

fn create_render_pass(device: &ash::Device, image_format: vk::Format) -> vk::RenderPass {
  // 1. define render pass to compile shader against
  let attachment = vk::AttachmentDescription::builder()
    .format(image_format)
    .samples(vk::SampleCountFlags::TYPE_1) // single sampled
    .load_op(vk::AttachmentLoadOp::CLEAR) // do not clear triangle background
    .store_op(vk::AttachmentStoreOp::STORE)
    // .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
    // .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
    // .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    // .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
    .build();

  let subpass_output_attachment = vk::AttachmentReference {
    attachment: 0, // from the array above
    layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
  };

  let subpass = vk::SubpassDescription::builder()
    // .flags(flags) // No values in vk?
    // .input_attachments(&[]) // INPUT: layout(input_attachment_index=X, set=Y, binding=Z)
    .color_attachments(&[subpass_output_attachment]) // OUTPUT
    // .depth_stencil_attachment(depth_stencil_attachment)
    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS) //
    // .preserve_attachments(preserve_attachments)
    // .resolve_attachments(resolve_attachments)
    .build();
  trace!("Subpass created, will be used to create render pass");

  let dependencies = vk::SubpassDependency::builder()
    .src_subpass(vk::SUBPASS_EXTERNAL)
    .dst_subpass(0)
    .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
    .src_access_mask(vk::AccessFlags::empty())
    .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
    .build();

  let create_info = vk::RenderPassCreateInfo::builder()
    // .flags(vk::RenderPassCreateFlags::) // some BS about rotation 90dgr?
    // .pCorrelatedViewMasks() // ?
    .dependencies(&[dependencies])
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

fn create_pipeline(
  device: &ash::Device,
  pipeline_cache: &vk::PipelineCache,
  render_pass: &vk::RenderPass,
) -> (vk::Pipeline, vk::PipelineLayout) {
  trace!("Will create pipeline for a (device, render pass) based on shaders");
  let attachement_count: usize = 1;

  // create shaders
  let (module_vs, stage_vs) = load_shader(
    device,
    vk::ShaderStageFlags::VERTEX,
    std::path::Path::new("./vert.spv"),
  );
  let (module_fs, stage_fs) = load_shader(
    device,
    vk::ShaderStageFlags::FRAGMENT,
    std::path::Path::new("./frag.spv"),
  );

  let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
  // TODO vertex desc here!
  .build();

  // DO NOT INLINE INTO .builder(), GPU CRASHED ON ME
  let dynamic_state = ps_dynamic_state(&[
    vk::DynamicState::VIEWPORT,
    vk::DynamicState::SCISSOR,
    // other: depth, stencil, blend etc.
  ]);

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
    .input_assembly_state(&ps_ia_triangle_list())
    // .tessellation_state(tessellation_state)
    .viewport_state(&ps_viewport_single_dynamic())
    .rasterization_state(&ps_raster_polygons())
    .multisample_state(&ps_multisample_disabled())
    .depth_stencil_state(&ps_depth_always_stencil_always())
    .color_blend_state(&ps_color_write_all(attachement_count))
    .dynamic_state(&dynamic_state)
    .layout(layout)
    .render_pass(*render_pass)
    // .subpass()
    // .base_pipeline_handle(base_pipeline_handle)
    // .base_pipeline_index(base_pipeline_index)
    .build();

  unsafe {
    let pipelines = device
      .create_graphics_pipelines(*pipeline_cache, &[create_info], None)
      .ok();
    device.destroy_shader_module(module_vs, None);
    device.destroy_shader_module(module_fs, None);
    match pipelines {
      Some(ps) if ps.len() > 0 => (*ps.first().unwrap(), layout),
      _ => panic!("Failed to create graphic pipeline"),
    }
  }
}

// https://github.com/MaikKlein/ash/blob/master/examples/src/lib.rs#L332
pub fn vk_init(window: &winit::window::Window) -> AppVk {
  let (entry, instance) = create_instance();
  let (debug_utils_loader, debug_messenger) = setup_debug_reporting(&entry, &instance);

  // surface data
  let surface_loader = Surface::new(&entry, &instance); // I guess some generic OS-independent thing?
  let surface_khr = unsafe { create_surface_khr(&entry, &instance, window) }; // real OS-backed thing

  // devices
  let (phys_device, queue_family_index) =
    pick_physical_device_and_queue_family_idx(&instance, &surface_loader, &surface_khr);
  let (device, queue) = pick_device_and_queue(&instance, &phys_device, queue_family_index);

  // swapchain - prepare
  let window_size = get_window_size(window);
  trace!("window_size {:?}", window_size);
  let swapchain_format = get_swapchain_format(&surface_loader, &surface_khr, &phys_device)
    .expect("Could not find valid surface format");
  let surface_capabilities = get_surface_capabilities(&phys_device, &surface_loader, &surface_khr);

  // swapchain
  let swapchain_loader = Swapchain::new(&instance, &device); // I guess some generic OS-independent thing?
  let swapchain = create_swapchain_khr(
    &swapchain_loader,
    &surface_khr,
    &swapchain_format,
    surface_capabilities,
    &window_size,
    queue_family_index,
  );
  let (swapchain_images, swapchain_image_views) = create_swapchain_images(
    &swapchain_loader,
    &swapchain,
    &device,
    swapchain_format.format,
  );
  let frames_in_flight = swapchain_images.len();
  info!("Will use {} frames in flight", frames_in_flight);

  // command buffers
  let cmd_pool = create_command_pool(&device, queue_family_index);
  let cmd_bufs = create_command_buffers(&device, cmd_pool, frames_in_flight);

  ///////////////////////////////////////
  ///////////////////////////////////////
  // TRIANGLE SPECIFIC STUFF STARTS HERE

  // render pass
  let render_pass = create_render_pass(&device, swapchain_format.format);

  // framebuffers
  let framebuffers = swapchain_image_views
    .iter()
    .map(|&iv| create_framebuffer(&device, &render_pass, &[iv], &window_size))
    .collect();

  // pipeline
  let pipeline_cache = create_pipeline_cache(&device);
  let (triangle_pipeline, layout) = create_pipeline(&device, &pipeline_cache, &render_pass);

  AppVk {
    entry,
    instance,
    swapchain: AppVkSwapchain {
      swapchain_loader,
      swapchain,
      size: window_size,
      framebuffers,
      image_views: swapchain_image_views,
      images: swapchain_images,
    },
    synchronize: AppVkSynchronize::new(&device, frames_in_flight),
    device: AppVkDevice {
      phys_device,
      queue_family_index,
      device,
      queue,
    },
    command_buffers: AppVkCommandBuffers {
      pool: cmd_pool,
      cmd_buffers: cmd_bufs,
    },
    pipelines: AppVkPipelines {
      pipeline_cache,
      pipeline_triangle: triangle_pipeline,
      pipeline_triangle_layout: layout,
    },
    render_passes: AppVkRenderPasses {
      render_pass_triangle: render_pass,
    },
    surface_loader,
    surface_khr,
    debug_utils_loader,
    debug_messenger,
  }
}
