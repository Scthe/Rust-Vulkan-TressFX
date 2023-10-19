use ash;
use ash::vk;
use log::info;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const BINDING_INDEX_PREV_PASS_RESULT: u32 = 0;

const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreenQuad.vert.spv",
  "./assets/shaders-compiled/present.frag.spv",
);

pub struct PresentPass {
  pub render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl PresentPass {
  pub fn new(vk_app: &VkCtx, image_format: vk::Format) -> Self {
    info!("Creating PresentPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = PresentPass::create_render_pass(device, image_format);
    let uniforms_desc = PresentPass::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout]);
    let pipeline =
      PresentPass::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

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

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![create_texture_binding(
      BINDING_INDEX_PREV_PASS_RESULT,
      vk::ShaderStageFlags::FRAGMENT,
    )]
  }

  fn create_pipeline(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    pipeline_layout: &vk::PipelineLayout,
  ) -> vk::Pipeline {
    let vertex_desc = ps_vertex_empty();

    create_pipeline_with_defaults(
      device,
      render_pass,
      pipeline_layout,
      SHADER_PATHS,
      vertex_desc,
      COLOR_ATTACHMENT_COUNT,
      |builder| {
        let pipeline_create_info = builder.build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
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
    exec_ctx: &PassExecContext,
    framebuffer: &vk::Framebuffer,
    previous_pass_render_result: &mut VkTexture,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    unsafe {
      self.cmd_resource_barriers(device, &command_buffer, previous_pass_render_result);

      // start render pass
      cmd_begin_render_pass_for_framebuffer(
        &device,
        &command_buffer,
        &self.render_pass,
        &framebuffer,
        &exec_ctx.size,
        &[],
      );
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // bind uniforms (do not move this)
      let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
      let uniform_resouces = [BindableResource::Texture {
        binding: BINDING_INDEX_PREV_PASS_RESULT,
        texture: previous_pass_render_result,
        sampler: vk_app.default_texture_sampler_nearest,
      }];
      bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);

      // draw calls
      cmd_draw_fullscreen_triangle(device, &command_buffer);

      // end
      device.cmd_end_render_pass(command_buffer)
    }
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    previous_pass_render_result: &mut VkTexture,
  ) {
    let texture_barrier = previous_pass_render_result.prepare_for_layout_transition(
      vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
      vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
      vk::AccessFlags::SHADER_READ,
    );
    device.cmd_pipeline_barrier(
      *command_buffer,
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      vk::PipelineStageFlags::FRAGMENT_SHADER,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[texture_barrier],
    );
  }
}
