use ash;
use ash::vk;
use log::info;

use crate::config::Config;
use crate::render_graph::forward_pass::ForwardPass;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_COLOR_SOURCE: u32 = 1;
const BINDING_INDEX_DEPTH: u32 = 2;

const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreenQuad.vert.spv",
  "./assets/shaders-compiled/sssBlur.frag.spv",
);

pub struct SSSBlurPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl SSSBlurPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating SSSBlurPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = SSSBlurPass::create_render_pass(device);
    let uniforms_desc = SSSBlurPass::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline =
      SSSBlurPass::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    SSSBlurPass {
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

  fn create_render_pass(device: &ash::Device) -> vk::RenderPass {
    let depth_attachment = create_depth_stencil_attachment(
      0,
      ForwardPass::DEPTH_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::LOAD,   // depth_load_op
      vk::AttachmentStoreOp::STORE, // depth_store_op
      vk::AttachmentLoadOp::LOAD,   // stencil_load_op
      vk::AttachmentStoreOp::STORE, // stencil_store_op
      vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    );
    let color_attachment = create_color_attachment(
      1,
      ForwardPass::DIFFUSE_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::LOAD,
      vk::AttachmentStoreOp::STORE,
      false,
    );

    // return unsafe {
    // create_render_pass_from_attachments(device, Some(depth_attachment), &[color_attachment])
    // };

    let subpass = vk::SubpassDescription::builder()
      .color_attachments(&[color_attachment.1])
      .depth_stencil_attachment(&depth_attachment.1)
      .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
      .build();

    let dependencies = vk::SubpassDependency::builder()
      .src_subpass(vk::SUBPASS_EXTERNAL)
      .dst_subpass(0)
      .src_stage_mask(
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
          | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
      )
      .src_access_mask(vk::AccessFlags::empty())
      .dst_stage_mask(
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
          | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
      )
      .dst_access_mask(
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
      )
      .build();

    let create_info = vk::RenderPassCreateInfo::builder()
      .dependencies(&[dependencies])
      .attachments(&[depth_attachment.0, color_attachment.0])
      .subpasses(&[subpass])
      .build();
    unsafe {
      device
        .create_render_pass(&create_info, None)
        .expect("Failed creating render pass")
    }
  }

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(BINDING_INDEX_CONFIG_UBO, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_COLOR_SOURCE, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_DEPTH, vk::ShaderStageFlags::FRAGMENT),
    ]
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
        let stencil_only_skin = ps_stencil_compare_equal(Config::STENCIL_BIT_SKIN);
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
          .depth_test_enable(false)
          .depth_write_enable(false)
          .depth_compare_op(vk::CompareOp::ALWAYS)
          .depth_bounds_test_enable(false)
          .stencil_test_enable(true)
          .front(stencil_only_skin)
          .back(stencil_only_skin)
          .build();

        let pipeline_create_info = builder.depth_stencil_state(&depth_stencil).build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    stencil_source_tex: &VkTexture,
    result_tex: &VkTexture,
  ) -> SSSBlurFramebuffer {
    let device = vk_app.vk_device();
    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[stencil_source_tex.image_view(), result_tex.image_view()],
      &result_tex.size(),
    );
    SSSBlurFramebuffer { fbo }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut SSSBlurFramebuffer,
    result_tex: &mut VkTexture,        // write
    depth_stencil_tex: &mut VkTexture, // write (stencil source)
    color_source_tex: &mut VkTexture,  // read
    linear_depth_tex: &mut VkTexture,  // read
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        result_tex,
        depth_stencil_tex,
        color_source_tex,
        linear_depth_tex,
      );

      // start render pass
      cmd_begin_render_pass_for_framebuffer(
        &device,
        &command_buffer,
        &self.render_pass,
        &framebuffer.fbo,
        &exec_ctx.size,
        &[], // TODO clear https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAttachmentLoadOp.html
      );
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // bind uniforms (do not move this)
      self.bind_uniforms(exec_ctx, color_source_tex, linear_depth_tex);

      // draw calls
      cmd_draw_fullscreen_triangle(device, &command_buffer);

      // end
      device.cmd_end_render_pass(command_buffer)
    }
  }

  unsafe fn bind_uniforms(
    &self,
    exec_ctx: &PassExecContext,
    color_source_tex: &mut VkTexture,
    linear_depth_tex: &mut VkTexture,
  ) {
    let vk_app = exec_ctx.vk_app;
    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);

    let uniform_resouces = [
      BindableResource::Uniform {
        binding: BINDING_INDEX_CONFIG_UBO,
        buffer: exec_ctx.config_buffer,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_COLOR_SOURCE,
        texture: color_source_tex,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_DEPTH,
        texture: linear_depth_tex,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
    ];
    bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    result_tex: &mut VkTexture,        // write
    depth_stencil_tex: &mut VkTexture, // write (stencil source)
    color_source_tex: &mut VkTexture,  // read
    linear_depth_tex: &mut VkTexture,  // read
  ) {
    let source_barrier = color_source_tex.barrier_prepare_attachment_for_shader_read();
    let linear_depth_barrier = linear_depth_tex.barrier_prepare_attachment_for_shader_read();
    device.cmd_pipeline_barrier(
      *command_buffer,
      // wait for previous use in:
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      // before we: execute fragment shader
      vk::PipelineStageFlags::FRAGMENT_SHADER,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[linear_depth_barrier, source_barrier],
    );

    let depth_barrier = depth_stencil_tex.barrier_prepare_attachment_for_write();
    let result_barrier = result_tex.barrier_prepare_attachment_for_write(); // we use stencil
    device.cmd_pipeline_barrier(
      *command_buffer,
      // wait for previous use in:
      vk::PipelineStageFlags::FRAGMENT_SHADER
        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
        | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
      // before we: execute stencil test or write output
      vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
        | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[result_barrier, depth_barrier],
    );
  }
}

pub struct SSSBlurFramebuffer {
  pub fbo: vk::Framebuffer,
}

impl SSSBlurFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    device.destroy_framebuffer(self.fbo, None);
  }
}
