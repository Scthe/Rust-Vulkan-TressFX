#![allow(dead_code)]
use ash;
use ash::version::DeviceV1_0;
use ash::vk;

use super::{create_viewport, size_to_rect_vk};

pub fn create_pipeline_cache(device: &ash::Device) -> vk::PipelineCache {
  let create_info = vk::PipelineCacheCreateInfo::builder().build();
  unsafe {
    device
      .create_pipeline_cache(&create_info, None)
      .expect("Failed to create pipeline cache")
  }
}

// This file contains presets for `vk::GraphicsPipelineCreateInfo`.
// Most common options, so it's actually manageable and <100LOC every time

/// PipelineInputAssembly-TRIANGLE_LIST
pub fn ps_ia_triangle_list() -> vk::PipelineInputAssemblyStateCreateInfo {
  vk::PipelineInputAssemblyStateCreateInfo::builder()
    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
    .build()
}

/// PipelineInputAssembly-TRIANGLE_FAN
pub fn ps_ia_triangle_fan() -> vk::PipelineInputAssemblyStateCreateInfo {
  vk::PipelineInputAssemblyStateCreateInfo::builder()
    .topology(vk::PrimitiveTopology::TRIANGLE_FAN)
    .build()
}

/// Hardcoded size, does not require PipelineDynamicStateCreateInfo later on
/// PipelineViewportState for viewports+scissors
pub fn ps_viewport_fill_rect(size: &vk::Extent2D) -> vk::PipelineViewportStateCreateInfo {
  let vp = create_viewport(&size);
  let scissors_rect = size_to_rect_vk(size);

  vk::PipelineViewportStateCreateInfo::builder()
    .viewports(&[vp])
    .scissors(&[scissors_rect])
    .build()
}

/// Does not specify dimensions during pipeline create, requires PipelineDynamicStateCreateInfo with
/// - vk::DynamicState::VIEWPORT
/// - vk::DynamicState::SCISSOR
pub fn ps_viewport_single_dynamic() -> vk::PipelineViewportStateCreateInfo {
  vk::PipelineViewportStateCreateInfo {
    viewport_count: 1,
    scissor_count: 1,
    ..Default::default()
  }
}

/// Default state that you would use to display opaque cube
pub fn ps_raster_polygons(
  cull_mode: vk::CullModeFlags,
) -> vk::PipelineRasterizationStateCreateInfo {
  vk::PipelineRasterizationStateCreateInfo::builder()
    .depth_clamp_enable(false) // when would You ever want it to be true?
    // .rasterizer_discard_enable(rasterizer_discard_enable)
    .polygon_mode(vk::PolygonMode::FILL)
    .cull_mode(cull_mode)
    .front_face(vk::FrontFace::COUNTER_CLOCKWISE) // TODO I don't remember OpenGL
    // .depth_bias_...
    .line_width(1.0) // validation layers: has to be 1.0 if not dynamic
    .build()
}

/// - Depth: test LESS, write ON
/// - Stencil: test SKIP
#[allow(dead_code)]
pub fn ps_depth_less_stencil_always() -> vk::PipelineDepthStencilStateCreateInfo {
  vk::PipelineDepthStencilStateCreateInfo::builder()
    .depth_test_enable(true)
    .depth_write_enable(true)
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
    .build()
}

/// - Depth: test SKIP, write OFF
/// - Stencil: test SKIP
pub fn ps_depth_always_stencil_always() -> vk::PipelineDepthStencilStateCreateInfo {
  vk::PipelineDepthStencilStateCreateInfo::builder()
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
    .build()
}

pub fn ps_multisample_disabled() -> vk::PipelineMultisampleStateCreateInfo {
  vk::PipelineMultisampleStateCreateInfo::builder()
    .rasterization_samples(vk::SampleCountFlags::TYPE_1)
    // fragment shader per sample? Yes, please do! Oh wait, validation layers..
    .sample_shading_enable(false)
    // other sample coverage stuff
    // other alpha to coverage stuff
    .build()
}

/// Write result to all color attachments, disable blending
fn ps_color_attachments_write_all(
  attachment_count: usize,
) -> Vec<vk::PipelineColorBlendAttachmentState> {
  // VULKAN SPEC:
  // > If the independent blending feature is not enabled on the device,
  // all VkPipelineColorBlendAttachmentState elements in the pAttachments
  // array must be identical.

  // PS. I always hated blend state
  let write_all = vk::PipelineColorBlendAttachmentState::builder()
    .color_write_mask(vk::ColorComponentFlags::all())
    .blend_enable(false)
    .build();

  let mut attachments =
    Vec::<vk::PipelineColorBlendAttachmentState>::with_capacity(attachment_count);
  for _i in 0..attachment_count {
    attachments.push(write_all);
  }

  attachments
}

/// Write result to all color attachments, disable blending
pub fn ps_color_blend_override(attachment_count: usize) -> vk::PipelineColorBlendStateCreateInfo {
  let color_attachments_write_all = ps_color_attachments_write_all(attachment_count);
  vk::PipelineColorBlendStateCreateInfo::builder()
    .attachments(&color_attachments_write_all)
    .build()
}

/// List of things that will be provided as separate command before draw (actuall 'runtime').
/// Used so that we do not have to specify everything during pipeline create
pub fn ps_dynamic_state(states: &[vk::DynamicState]) -> vk::PipelineDynamicStateCreateInfo {
  vk::PipelineDynamicStateCreateInfo::builder()
    .dynamic_states(states)
    .build()
}
