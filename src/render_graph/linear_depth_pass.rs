use ash;
use ash::vk;
use log::info;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_SCENE_DEPTH: u32 = 1;

const RESULT_TEXTURE_FORMAT: vk::Format = vk::Format::R32_SFLOAT;
const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreenQuad.vert.spv",
  "./assets/shaders-compiled/linearDepth.frag.spv",
);

// TODO verify
// TODO is this pass even used? What to display in the preview?
pub struct LinearDepthPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl LinearDepthPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating LinearDepthPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = LinearDepthPass::create_render_pass(device);
    let uniforms_desc = LinearDepthPass::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline =
      LinearDepthPass::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    LinearDepthPass {
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
    let color_attachment = create_color_attachment(
      0,
      RESULT_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::DONT_CARE, // we override every pixel regardless
      vk::AttachmentStoreOp::STORE,
      false,
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
    vec![
      create_ubo_binding(BINDING_INDEX_CONFIG_UBO, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_SCENE_DEPTH, vk::ShaderStageFlags::FRAGMENT),
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
        let pipeline_create_info = builder.build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    frame_id: usize,
    size: &vk::Extent2D,
  ) -> LinearDepthPassFramebuffer {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    let linear_depth_tex = VkTexture::empty(
      device,
      allocator,
      vk_app,
      format!("LinearDepthPass.linear_depth#{}", frame_id),
      *size,
      RESULT_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
      vk::MemoryPropertyFlags::DEVICE_LOCAL,
      vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    );

    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[linear_depth_tex.image_view()],
      &size,
    );

    LinearDepthPassFramebuffer {
      linear_depth_tex,
      fbo,
    }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut LinearDepthPassFramebuffer,
    depth_stencil_tex: &mut VkTexture,
    depth_tex_image_view: vk::ImageView,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    unsafe {
      self.cmd_resource_barriers(device, &command_buffer, framebuffer, depth_stencil_tex);

      // start render pass
      cmd_begin_render_pass_for_framebuffer(
        &device,
        &command_buffer,
        &self.render_pass,
        &framebuffer.fbo,
        &exec_ctx.size,
        &[],
      );
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // bind uniforms (do not move this)
      self.bind_uniforms(exec_ctx, depth_stencil_tex, depth_tex_image_view);

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
    framebuffer: &mut LinearDepthPassFramebuffer,
    depth_stencil_tex: &mut VkTexture,
  ) {
    let depth_barrier = depth_stencil_tex.barrier_prepare_attachment_for_shader_read();
    device.cmd_pipeline_barrier(
      *command_buffer,
      // wait for previous use in:
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
        | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
      // before we: execute fragment shader
      vk::PipelineStageFlags::FRAGMENT_SHADER,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[depth_barrier],
    );

    let result_barrier = framebuffer
      .linear_depth_tex
      .barrier_prepare_attachment_for_write();
    device.cmd_pipeline_barrier(
      *command_buffer,
      // wait for previous use in:
      vk::PipelineStageFlags::FRAGMENT_SHADER,
      // before we: write
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[result_barrier],
    );
  }

  unsafe fn bind_uniforms(
    &self,
    exec_ctx: &PassExecContext,
    depth_stencil_tex: &mut VkTexture,
    depth_tex_image_view: vk::ImageView,
  ) {
    let vk_app = exec_ctx.vk_app;
    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);

    let uniform_resouces = [
      BindableResource::Uniform {
        binding: BINDING_INDEX_CONFIG_UBO,
        buffer: exec_ctx.config_buffer,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SCENE_DEPTH,
        texture: depth_stencil_tex,
        image_view: Some(depth_tex_image_view),
        sampler: vk_app.default_texture_sampler_nearest,
      },
    ];
    bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);
  }
}

pub struct LinearDepthPassFramebuffer {
  /// Values are negative due to Vulkan coordinate system
  pub linear_depth_tex: VkTexture,
  pub fbo: vk::Framebuffer,
}

impl LinearDepthPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    self.linear_depth_tex.delete(device, allocator);
  }
}
