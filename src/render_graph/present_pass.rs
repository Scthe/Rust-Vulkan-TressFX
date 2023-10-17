use ash;
use ash::vk;
use log::trace;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

pub const BINDING_INDEX_PREV_PASS_RESULT: u32 = 0;

pub struct PresentPass {
  pub render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl PresentPass {
  pub fn new(vk_app: &VkCtx, image_format: vk::Format) -> Self {
    trace!("Creating PresentPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = PresentPass::create_render_pass(device, image_format);
    let uniforms_layout = PresentPass::create_uniforms_layout(device);
    let (pipeline, pipeline_layout) =
      PresentPass::create_pipeline(device, pipeline_cache, &render_pass, &[uniforms_layout]);

    PresentPass {
      render_pass,
      pipeline,
      pipeline_layout,
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
    let color_attachment = create_color_attachment(
      0,
      image_format,
      vk::AttachmentLoadOp::DONT_CARE, // we override every pixel regardless
      vk::AttachmentStoreOp::STORE,
      true,
    );

    let subpass = vk::SubpassDescription::builder()
      .color_attachments(&[color_attachment.1])
      .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
      .build();

    // TODO dependencies
    let dependencies = vk::SubpassDependency::builder()
      .src_subpass(vk::SUBPASS_EXTERNAL)
      .dst_subpass(0)
      .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
      .src_access_mask(vk::AccessFlags::empty())
      .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
      .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
      .build();

    let create_info = vk::RenderPassCreateInfo::builder()
      .dependencies(&[dependencies])
      .attachments(&[color_attachment.0])
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
    let binding_diff_tex = create_texture_binding(
      BINDING_INDEX_PREV_PASS_RESULT,
      vk::ShaderStageFlags::FRAGMENT,
    );

    let ubo_descriptors_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
      .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR)
      .bindings(&[binding_diff_tex])
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
    // pipeline layout
    let pipeline_layout = create_pipeline_layout(device, uniform_layouts);

    // create shaders
    let (module_vs, stage_vs, module_fs, stage_fs) = load_render_shaders(
      device,
      "./assets/shaders-compiled/present.vert.spv",
      "./assets/shaders-compiled/present.frag.spv",
    );

    let vertex_input_state = ps_vertex_empty();

    let dynamic_state = ps_dynamic_state(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    // create pipeline itself
    // TODO create util for this in `pipeline.rs` for fullscreen quad.
    //      Due to references may have to provide `modify_pipeline_create_info`?
    //      pub fn create_fs_quad_pipeline(..., modify_pipeline_create_info: FnOne(vk::GraphicsPipelineCreateInfo)) -> vk::Pipeline
    let color_attachment_count: usize = 1;
    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
      .stages(&[stage_vs, stage_fs])
      .vertex_input_state(&vertex_input_state)
      .input_assembly_state(&ps_ia_triangle_list())
      .viewport_state(&ps_viewport_single_dynamic())
      .rasterization_state(&ps_raster_polygons(vk::CullModeFlags::NONE))
      .multisample_state(&ps_multisample_disabled())
      .depth_stencil_state(&ps_depth_always_stencil_always())
      .color_blend_state(&ps_color_blend_override(color_attachment_count))
      .dynamic_state(&dynamic_state)
      .layout(pipeline_layout)
      .render_pass(*render_pass)
      .build();

    let pipeline = create_pipeline(device, pipeline_cache, pipeline_create_info);

    unsafe {
      device.destroy_shader_module(module_vs, None);
      device.destroy_shader_module(module_fs, None);
    }

    (pipeline, pipeline_layout)
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    swapchain_image_view: vk::ImageView,
    size: &vk::Extent2D,
  ) -> vk::Framebuffer {
    let device = vk_app.vk_device();
    create_framebuffer(device, self.render_pass, &[swapchain_image_view], &size)
  }

  pub fn execute(
    &self,
    vk_app: &VkCtx,
    command_buffer: vk::CommandBuffer,
    framebuffer: &vk::Framebuffer,
    size: vk::Extent2D,
    previous_pass_render_result: &mut VkTexture,
  ) -> () {
    let device = vk_app.vk_device();
    let push_descriptor = &vk_app.push_descriptor;
    let render_area = size_to_rect_vk(&size);
    let viewport = create_viewport(&size);

    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
      .render_pass(self.render_pass)
      .framebuffer(*framebuffer)
      .render_area(render_area)
      .build();

    unsafe {
      let texture_barrier = previous_pass_render_result.prepare_for_layout_transition(
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        vk::AccessFlags::SHADER_READ,
      );
      device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[texture_barrier],
      );

      // start render pass
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

      // bind uniforms
      let resouce_binder = ResouceBinder {
        push_descriptor,
        command_buffer,
        pipeline_layout: self.pipeline_layout,
      };
      let uniform_resouces = [BindableResource::Texture {
        binding: BINDING_INDEX_PREV_PASS_RESULT,
        texture: previous_pass_render_result,
        sampler: vk_app.default_texture_sampler_nearest,
      }];
      bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);

      // draw calls
      // TODO vk_app.cmd_draw_fullscreen_quad();
      device.cmd_draw(command_buffer, 3, 1, 0, 0); // 3 vertices (1 triangle), 1 instance, no special offset

      // end
      device.cmd_end_render_pass(command_buffer)
    }
  }
}
