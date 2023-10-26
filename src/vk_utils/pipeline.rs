#![allow(dead_code)]
use ash;
use ash::vk;

use super::{create_viewport, load_render_shaders, size_to_rect_vk};

pub fn create_pipeline_cache(device: &ash::Device) -> vk::PipelineCache {
  let create_info = vk::PipelineCacheCreateInfo::builder().build();
  unsafe {
    device
      .create_pipeline_cache(&create_info, None)
      .expect("Failed to create pipeline cache")
  }
}

pub fn create_pipeline_layout(
  device: &ash::Device,
  uniform_layouts: &[vk::DescriptorSetLayout],
  push_constant_ranges: &[vk::PushConstantRange],
) -> vk::PipelineLayout {
  let create_info = vk::PipelineLayoutCreateInfo::builder()
    .set_layouts(uniform_layouts)
    .push_constant_ranges(push_constant_ranges)
    .build();
  unsafe {
    device
      .create_pipeline_layout(&create_info, None)
      .expect("Failed to create pipeline layout")
  }
}

pub fn create_pipeline(
  device: &ash::Device,
  pipeline_cache: &vk::PipelineCache,
  pipeline_create_info: vk::GraphicsPipelineCreateInfo,
) -> vk::Pipeline {
  let pipelines = unsafe {
    device
      .create_graphics_pipelines(*pipeline_cache, &[pipeline_create_info], None)
      .ok()
  };
  match pipelines {
    Some(ps) if ps.len() > 0 => *ps.first().unwrap(),
    _ => panic!("Failed to create graphic pipeline"),
  }
}

// Alternatively, create PipelineBuilderWithDefaults that wraps `vk::GraphicsPipelineCreateInfoBuilder` and provides 2-3 fns that actually are used
pub fn create_pipeline_with_defaults(
  device: &ash::Device,
  render_pass: &vk::RenderPass,
  pipeline_layout: &vk::PipelineLayout,
  shader_paths: (&str, &str),
  vertex_desc: vk::PipelineVertexInputStateCreateInfo,
  color_attachment_count: usize,
  creator: impl Fn(vk::GraphicsPipelineCreateInfoBuilder) -> vk::Pipeline,
) -> vk::Pipeline {
  let (module_vs, stage_vs, module_fs, stage_fs) =
    load_render_shaders(device, shader_paths.0, shader_paths.1);

  let dynamic_state = ps_dynamic_state(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

  // create pipeline itself
  let stages = [stage_vs, stage_fs];
  let input_assembly_state = ps_ia_triangle_list();
  let viewport_state = ps_viewport_single_dynamic();
  let rasterization_state = ps_raster_polygons(vk::CullModeFlags::NONE);
  let multisample_state = ps_multisample_disabled();
  let depth_stencil_state = ps_depth_always_stencil_always();
  let color_blend_state = ps_color_blend_override(color_attachment_count);
  let create_info_builder = vk::GraphicsPipelineCreateInfo::builder()
    .stages(&stages)
    .vertex_input_state(&vertex_desc)
    .input_assembly_state(&input_assembly_state)
    .viewport_state(&viewport_state)
    .rasterization_state(&rasterization_state)
    .multisample_state(&multisample_state)
    .depth_stencil_state(&depth_stencil_state)
    .color_blend_state(&color_blend_state)
    .dynamic_state(&dynamic_state)
    .layout(*pipeline_layout)
    .render_pass(*render_pass);

  let pipeline = creator(create_info_builder);

  unsafe {
    device.destroy_shader_module(module_vs, None);
    device.destroy_shader_module(module_fs, None);
  }

  pipeline
}

// This file contains presets for `vk::GraphicsPipelineCreateInfo`.
// Most common options, so it's actually manageable and <100LOC every time

/// No data for vertices provided by the app, it will all be handled in the shader.
/// Common usage is
/// https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/
pub fn ps_vertex_empty() -> vk::PipelineVertexInputStateCreateInfo {
  let mut info = vk::PipelineVertexInputStateCreateInfo::builder().build();
  info.vertex_attribute_description_count = 0;
  info.vertex_binding_description_count = 0;
  info
}

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
    .color_write_mask(vk::ColorComponentFlags::RGBA)
    .blend_enable(false)
    .src_color_blend_factor(vk::BlendFactor::ONE) // shader output
    .dst_color_blend_factor(vk::BlendFactor::ZERO) // existing value on destination attachment
    .src_alpha_blend_factor(vk::BlendFactor::ONE) // shader output
    .dst_alpha_blend_factor(vk::BlendFactor::ZERO) // existing value on destination attachment
    .build();

  let mut attachments =
    Vec::<vk::PipelineColorBlendAttachmentState>::with_capacity(attachment_count);
  for _i in 0..attachment_count {
    attachments.push(write_all);
  }

  attachments
}

/// Write result to all color attachments, disable blending
pub fn ps_color_blend_override(
  color_attachment_count: usize,
) -> vk::PipelineColorBlendStateCreateInfo {
  let color_attachments_write_all = ps_color_attachments_write_all(color_attachment_count);
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
