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
  "./assets/shaders-compiled/fullscreen_quad.vert.spv",
  "./assets/shaders-compiled/linear_depth.frag.spv",
);

/// Used as precomputed value (in [-z_near, -z_far] range). Sometimes we need
/// original depth/stencil buffer attached to framebuffer (so in write mode),
/// but still want to read depth value in shader. So linear depth functions
/// just as a copy with nicer-to-use format (no perspective-float distortion).
///
/// Usage example: SSSBlur pass
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

    let render_pass = Self::create_render_pass(device);
    let uniforms_desc = Self::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline = Self::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    Self {
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

    unsafe { create_render_pass_from_attachments(device, None, &[color_attachment]) }
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

    let linear_depth_tex = vk_app.create_texture_empty(
      format!("LinearDepthPass.linear_depth#{}", frame_id),
      *size,
      RESULT_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
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
    VkTexture::cmd_transition_attachments_for_read_barrier(
      device,
      *command_buffer,
      &mut [depth_stencil_tex],
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
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
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
