use ash;
use ash::extensions::khr::PushDescriptor;
use ash::vk;
use log::trace;

use crate::render_graph::_shared::RenderableVertex;
use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

/// Texture binding in set 1.
/// must match in-shader
pub const MODEL_TEXTURE_BINDING_INDEX: u32 = 0;

pub struct ForwardPass {
  pub render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  // descriptor_sets: Vec<vk::DescriptorSet>,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl ForwardPass {
  pub fn new(
    vk_app: &VkCtx,
    image_format: vk::Format,
    config_uniforms_layout: vk::DescriptorSetLayout,
  ) -> Self {
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = ForwardPass::create_render_pass(device, image_format);
    let uniforms_layout = ForwardPass::create_uniforms_layout(device);
    let (pipeline, pipeline_layout) = ForwardPass::create_pipeline(
      device,
      pipeline_cache,
      &render_pass,
      &[config_uniforms_layout, uniforms_layout],
    );

    /*
    // descriptor sets (uniforms) - one per frame in flight
    let descriptor_sets = unsafe {
      create_descriptor_sets(
        device,
        &vk_app.descriptor_pool,
        vk_app.frames_in_flight(),
        &uniforms_layout,
      )
    };
    */

    ForwardPass {
      render_pass,
      pipeline,
      pipeline_layout,
      // descriptor_sets,
      uniforms_layout,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_descriptor_set_layout(self.uniforms_layout, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
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

  fn create_uniforms_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    let texture_binding =
      create_texture_layout(MODEL_TEXTURE_BINDING_INDEX, vk::ShaderStageFlags::FRAGMENT);

    let ubo_descriptors_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
      .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR)
      .bindings(&[texture_binding])
      .build();

    unsafe {
      device
        .create_descriptor_set_layout(&ubo_descriptors_create_info, None)
        .expect("Failed to create DescriptorSetLayout")
    }
  }

  fn create_pipeline(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    uniform_layouts: &[vk::DescriptorSetLayout],
  ) -> (vk::Pipeline, vk::PipelineLayout) {
    trace!("Will create pipeline for a (device, render pass) based on shaders");
    let attachment_count: usize = 1;

    // pipeline layout
    let create_info = vk::PipelineLayoutCreateInfo::builder()
      .set_layouts(uniform_layouts)
      .build();
    let pipeline_layout = unsafe {
      device
        .create_pipeline_layout(&create_info, None)
        .expect("Failed to create pipeline layout")
    };

    // create shaders
    let (module_vs, stage_vs) = load_shader(
      device,
      vk::ShaderStageFlags::VERTEX,
      std::path::Path::new("./assets/shaders-compiled/triangle.vert.spv"),
    );
    let (module_fs, stage_fs) = load_shader(
      device,
      vk::ShaderStageFlags::FRAGMENT,
      std::path::Path::new("./assets/shaders-compiled/triangle.frag.spv"),
    );

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
      .vertex_attribute_descriptions(&RenderableVertex::get_attributes_descriptions())
      .vertex_binding_descriptions(&RenderableVertex::get_bindings_descriptions())
      .build();

    let dynamic_state = ps_dynamic_state(&[
      vk::DynamicState::VIEWPORT,
      vk::DynamicState::SCISSOR,
      // other: depth, stencil, blend etc.
    ]);

    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
      // .flags(vk::PipelineCreateFlags::)
      .stages(&[stage_vs, stage_fs])
      .vertex_input_state(&vertex_input_state)
      .input_assembly_state(&ps_ia_triangle_list())
      // .tessellation_state(tessellation_state)
      .viewport_state(&ps_viewport_single_dynamic())
      .rasterization_state(&ps_raster_polygons(vk::CullModeFlags::NONE)) // TODO cull backfaces
      .multisample_state(&ps_multisample_disabled())
      .depth_stencil_state(&ps_depth_always_stencil_always())
      .color_blend_state(&ps_color_blend_override(attachment_count))
      .dynamic_state(&dynamic_state)
      .layout(pipeline_layout)
      .render_pass(*render_pass)
      // .subpass()
      // .base_pipeline_handle(base_pipeline_handle)
      // .base_pipeline_index(base_pipeline_index)
      .build();

    let pipelines = unsafe {
      let pipelines = device
        .create_graphics_pipelines(*pipeline_cache, &[pipeline_create_info], None)
        .ok();
      device.destroy_shader_module(module_vs, None);
      device.destroy_shader_module(module_fs, None);
      pipelines
    };
    let pipeline = match pipelines {
      Some(ps) if ps.len() > 0 => *ps.first().unwrap(),
      _ => panic!("Failed to create graphic pipeline"),
    };

    (pipeline, pipeline_layout)
  }

  /*
  pub fn bind_data_to_descriptors(
    &self,
    in_flight_frame_idx: usize,
    vk_app: &VkCtx,
    scene: &World,
  ) {
    let device = vk_app.vk_device();
    let descriptor_set = self.descriptor_set(in_flight_frame_idx);

    let diffuse_texture = BindableResource::Texture {
      descriptor_set,
      binding: MODEL_TEXTURE_BINDING_INDEX,
      texture: &scene.test_texture,
      sampler: vk_app.default_texture_sampler,
    };

    unsafe {
      bind_resources_to_descriptors(device, &[diffuse_texture]);
    };
  }
  */

  pub fn execute(
    &self,
    vk_app: &VkCtx,
    scene: &World,
    command_buffer: vk::CommandBuffer,
    framebuffer: vk::Framebuffer,
    size: vk::Extent2D,
    config_buffer: &VkBuffer,
  ) -> () {
    let device = vk_app.vk_device();
    let push_descriptor = &vk_app.push_descriptor;
    let render_area = size_to_rect_vk(&size);
    let viewport = create_viewport(&size);
    let clear_color = vk::ClearColorValue {
      float32: [0.2f32, 0.2f32, 0.2f32, 1f32],
    };
    // let descriptor_set = self.descriptor_set(in_flight_frame_idx);

    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
      .render_pass(self.render_pass)
      .framebuffer(framebuffer)
      .render_area(render_area)
      .clear_values(&[vk::ClearValue { color: clear_color }])
      .build();

    // TODO no need to rerecord every frame TBH. Everything can be controlled by uniforms etc.
    unsafe {
      /*
      device.cmd_pipeline_barrier(
        *command_buffer,
        vk::PipelineStageFlags::ALL_GRAPHICS,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dependency_flags,
        memory_barriers,
        buffer_memory_barriers,
        image_memory_barriers,
      );
      */

      device.cmd_begin_render_pass(
        command_buffer,
        &render_pass_begin_info,
        vk::SubpassContents::INLINE,
      );

      // draw calls go here
      device.cmd_set_viewport(command_buffer, 0, &[viewport]);
      device.cmd_set_scissor(command_buffer, 0, &[render_area]);
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );
      /*
      device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline_layout,
        0,
        &[config_descriptor_set, descriptor_set],
        &[],
      );
      */
      // here
      let config_buf_info = vk::DescriptorBufferInfo {
        buffer: config_buffer.buffer,
        offset: 0,
        range: vk::WHOLE_SIZE, // or buffer.size
      };
      let config_binding = vk::WriteDescriptorSet::builder()
        // .dst_set(*descriptor_set)
        .dst_binding(0) // !
        .dst_array_element(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .buffer_info(&[config_buf_info])
        .build();
      push_descriptor.cmd_push_descriptor_set(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline_layout,
        0, // set!
        &[config_binding],
      );
      let tex_info = vk::DescriptorImageInfo {
        image_layout: scene.test_texture.layout,
        image_view: scene.test_texture.image_view(),
        sampler: vk_app.default_texture_sampler,
      };
      let tex_binding = vk::WriteDescriptorSet::builder()
        // .dst_set(*descriptor_set)
        .dst_binding(0) // !
        .dst_array_element(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(&[tex_info])
        .build();
      push_descriptor.cmd_push_descriptor_set(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline_layout,
        1, // set!
        &[tex_binding],
      );

      for entity in &scene.entities {
        device.cmd_bind_vertex_buffers(command_buffer, 0, &[entity.vertex_buffer.buffer], &[0]);
        device.cmd_bind_index_buffer(
          command_buffer,
          entity.index_buffer.buffer,
          0,
          vk::IndexType::UINT32,
        );
        device.cmd_draw_indexed(command_buffer, entity.vertex_count, 1, 0, 0, 0);
      }

      device.cmd_end_render_pass(command_buffer)
    }
  }

  // fn descriptor_set(&self, swapchain_image_index: usize) -> vk::DescriptorSet {
  // self.descriptor_sets[swapchain_image_index]
  // }
}
