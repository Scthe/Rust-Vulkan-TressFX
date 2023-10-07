use ash;
use ash::version::DeviceV1_0;
use ash::vk;
use bytemuck;
use glam::Vec4;
use log::trace;

use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::misc::SceneUniformBuffer;

#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct TriangleVertex {
  pos: Vec4, // TODO Vec2, Vec3 are enough
  color: Vec4,
}
unsafe impl bytemuck::Zeroable for TriangleVertex {}
unsafe impl bytemuck::Pod for TriangleVertex {}

impl TriangleVertex {
  pub fn new(pos: (f32, f32), col: (f32, f32, f32)) -> TriangleVertex {
    TriangleVertex {
      pos: Vec4::new(pos.0, pos.1, 0.0f32, 1.0f32),
      color: Vec4::new(col.0, col.1, col.2, 1.0f32),
    }
  }

  fn get_bindings_descriptions() -> [vk::VertexInputBindingDescription; 1] {
    [vk::VertexInputBindingDescription {
      binding: 0,
      input_rate: vk::VertexInputRate::VERTEX,
      stride: std::mem::size_of::<TriangleVertex>() as u32,
    }]
  }

  fn get_attributes_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
    [
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 0,
        format: vk::Format::R32G32_SFLOAT,
        offset: 0, // offsetof(TriangleVertex, pos),
      },
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 1,
        format: vk::Format::R32G32B32_SFLOAT,
        offset: std::mem::size_of::<Vec4>() as u32, // offsetted by 'position' from beginning of structure
      },
    ]
  }
}

pub struct ForwardPass {
  pub render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
}

impl ForwardPass {
  pub fn new(vk_app: &VkCtx, image_format: vk::Format) -> Self {
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = ForwardPass::create_render_pass(device, image_format);
    let pipeline_layout = ForwardPass::create_pipeline_layout(device);
    let pipeline =
      ForwardPass::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    ForwardPass {
      render_pass,
      pipeline,
      pipeline_layout,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass, None);
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

  fn create_pipeline_layout(device: &ash::Device) -> vk::PipelineLayout {
    let scene_ubo = SceneUniformBuffer::get_layout(device);

    // texture/buffer bindings
    let create_info = vk::PipelineLayoutCreateInfo::builder()
      .set_layouts(&[scene_ubo])
      .build();
    let pipeline_layout = unsafe {
      device
        .create_pipeline_layout(&create_info, None)
        .expect("Failed to create pipeline layout")
    };

    pipeline_layout
  }

  fn create_pipeline(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    pipeline_layout: &vk::PipelineLayout,
  ) -> vk::Pipeline {
    trace!("Will create pipeline for a (device, render pass) based on shaders");
    let attachment_count: usize = 1;

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
      .vertex_attribute_descriptions(&TriangleVertex::get_attributes_descriptions())
      .vertex_binding_descriptions(&TriangleVertex::get_bindings_descriptions())
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
      .layout(*pipeline_layout)
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
    match pipelines {
      Some(ps) if ps.len() > 0 => *ps.first().unwrap(),
      _ => panic!("Failed to create graphic pipeline"),
    }
  }

  pub fn execute(
    &self,
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    scene: &World,
    framebuffer: vk::Framebuffer,
    descriptor_set: vk::DescriptorSet,
    size: vk::Extent2D,
  ) -> () {
    let render_area = size_to_rect_vk(&size);
    let viewport = create_viewport(&size);
    let clear_color = vk::ClearColorValue {
      float32: [0.2f32, 0.2f32, 0.2f32, 1f32],
    };

    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
      .render_pass(self.render_pass)
      .framebuffer(framebuffer)
      .render_area(render_area)
      .clear_values(&[vk::ClearValue { color: clear_color }])
      .build();

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
      device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline_layout,
        0,
        &[descriptor_set],
        &[],
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
}
