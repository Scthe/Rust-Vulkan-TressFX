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

/// `vk::GraphicsPipelineCreateInfoBuilder` takes references, and we cannot return
/// references to stack allocated values. Thus, we will use closure.
///
/// Alternatively, we could create `PipelineBuilderWithDefaults` class that wraps
/// `vk::GraphicsPipelineCreateInfoBuilder` and provides access to internal raw builder.
/// The references will be to struct's fields, so still alive. Though closures are less
/// verbose and let's be honest: classes and closures are the same thing.
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

  let mut attachment_blends =
    Vec::<vk::PipelineColorBlendAttachmentState>::with_capacity(color_attachment_count);

  // create pipeline itself
  let stages = [stage_vs, stage_fs];
  let input_assembly_state = ps_ia_triangle_list();
  let viewport_state = ps_viewport_single_dynamic();
  let rasterization_state = ps_raster_polygons(vk::CullModeFlags::NONE);
  let multisample_state = ps_multisample_disabled();
  let depth_stencil_state = ps_depth_always_stencil_always();
  let color_blend_state = ps_color_blend_override(
    &mut attachment_blends,
    color_attachment_count,
    vk::ColorComponentFlags::RGBA,
  );
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
    .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
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

pub fn ps_stencil_write_if_touched(reference: u32, override_current: bool) -> vk::StencilOpState {
  let write_mask = if override_current {
    reference
  } else {
    0xffffffff
  };
  vk::StencilOpState {
    // do not skip fields! Rust defaults masks to 0, so things do not work as expected
    pass_op: vk::StencilOp::REPLACE,
    depth_fail_op: vk::StencilOp::REPLACE,
    fail_op: vk::StencilOp::REPLACE,
    reference,
    write_mask,
    compare_mask: 0xffffffff,
    compare_op: vk::CompareOp::ALWAYS,
  }
}

pub fn ps_stencil_write_if_depth_passed(
  reference: u32,
  override_current: bool,
) -> vk::StencilOpState {
  let write_mask = if override_current {
    reference
  } else {
    0xffffffff
  };
  vk::StencilOpState {
    // do not skip fields! Rust defaults masks to 0, so things do not work as expected
    pass_op: vk::StencilOp::REPLACE,
    depth_fail_op: vk::StencilOp::KEEP,
    fail_op: vk::StencilOp::KEEP,
    reference,
    write_mask,
    compare_mask: 0xffffffff,
    compare_op: vk::CompareOp::ALWAYS,
  }
}

pub fn ps_stencil_compare_equal(reference: u32) -> vk::StencilOpState {
  vk::StencilOpState {
    // do not skip fields! Rust defaults masks to 0, so things do not work as expected
    reference,
    compare_op: vk::CompareOp::EQUAL,
    pass_op: vk::StencilOp::KEEP,
    depth_fail_op: vk::StencilOp::KEEP,
    fail_op: vk::StencilOp::KEEP,
    write_mask: 0,
    compare_mask: reference,
  }
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
pub fn ps_color_attachment_override(
  color_write_mask: vk::ColorComponentFlags,
) -> vk::PipelineColorBlendAttachmentState {
  // PS. I always hated blend state
  vk::PipelineColorBlendAttachmentState::builder()
    .color_write_mask(color_write_mask)
    .blend_enable(false)
    .src_color_blend_factor(vk::BlendFactor::ONE) // shader output
    .dst_color_blend_factor(vk::BlendFactor::ZERO) // existing value on destination attachment
    .src_alpha_blend_factor(vk::BlendFactor::ONE) // shader output
    .dst_alpha_blend_factor(vk::BlendFactor::ZERO) // existing value on destination attachment
    .build()
}

/// Copy blends state `color_attachment_count` times.
///
/// **VULKAN SPEC:**
/// > If the independent blending feature is not enabled on the device,
/// all VkPipelineColorBlendAttachmentState elements in the pAttachments
/// array must be identical.
pub fn ps_color_blend_state(
  attachment_blends: &mut Vec<vk::PipelineColorBlendAttachmentState>, // needed cause `PipelineColorBlendAttachmentState` has ref
  color_attachment_count: usize,
  blend_state: vk::PipelineColorBlendAttachmentState,
) -> vk::PipelineColorBlendStateCreateInfo {
  for _i in 0..color_attachment_count {
    attachment_blends.push(blend_state);
  }

  vk::PipelineColorBlendStateCreateInfo::builder()
    .attachments(&attachment_blends)
    .build()
}

/// Write result to all color attachments, disable blending
pub fn ps_color_blend_override(
  attachment_blends: &mut Vec<vk::PipelineColorBlendAttachmentState>,
  color_attachment_count: usize,
  color_write_mask: vk::ColorComponentFlags,
) -> vk::PipelineColorBlendStateCreateInfo {
  let blend_state = ps_color_attachment_override(color_write_mask);
  ps_color_blend_state(attachment_blends, color_attachment_count, blend_state)
}

/// List of things that will be provided as separate command before draw (actuall 'runtime').
/// Used so that we do not have to specify everything during pipeline create
pub fn ps_dynamic_state(states: &[vk::DynamicState]) -> vk::PipelineDynamicStateCreateInfo {
  vk::PipelineDynamicStateCreateInfo::builder()
    .dynamic_states(states)
    .build()
}
