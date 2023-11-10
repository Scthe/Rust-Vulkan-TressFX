use std::mem::size_of;

use ash;
use ash::vk;
use glam::{vec2, Vec2};
use log::info;

use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_COLOR_SOURCE: u32 = 1;
const BINDING_INDEX_LINEAR_DEPTH: u32 = 2;

const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreen_quad.vert.spv",
  "./assets/shaders-compiled/blur.frag.spv",
);

/// Depth-aware blur. Rejects contribution from samples that are too far in world space
/// from the center pixel. Implemented as 2 passes - 1nd horizontal and 2nd vertical.
pub struct BlurPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl BlurPass {
  pub const BLUR_DIRECTION_PASS0: Vec2 = vec2(1.0, 0.0);
  pub const BLUR_DIRECTION_PASS1: Vec2 = vec2(0.0, 1.0);

  pub fn new(vk_app: &VkCtx, format: vk::Format) -> Self {
    info!("Creating BlurPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = Self::create_render_pass(device, format);
    let uniforms_desc = Self::get_uniforms_layout();
    let push_constant_ranges = Self::get_push_constant_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout =
      create_pipeline_layout(device, &[uniforms_layout], &[push_constant_ranges]);
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

  fn create_render_pass(device: &ash::Device, format: vk::Format) -> vk::RenderPass {
    let color_attachment = create_color_attachment(
      0,
      format,
      vk::AttachmentLoadOp::DONT_CARE,
      vk::AttachmentStoreOp::STORE,
      false,
    );

    unsafe { create_render_pass_from_attachments(device, None, &[color_attachment]) }
  }

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(BINDING_INDEX_CONFIG_UBO, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_COLOR_SOURCE, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_LINEAR_DEPTH, vk::ShaderStageFlags::FRAGMENT),
    ]
  }

  fn get_push_constant_layout() -> vk::PushConstantRange {
    vk::PushConstantRange::builder()
      .offset(0)
      .size(size_of::<BlurPassPushConstants>() as _)
      .stage_flags(vk::ShaderStageFlags::FRAGMENT)
      .build()
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

  pub fn create_framebuffer(&self, vk_app: &VkCtx, result_tex: &VkTexture) -> BlurFramebuffer {
    let device = vk_app.vk_device();
    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[result_tex.image_view()],
      &result_tex.size(),
    );
    BlurFramebuffer { fbo }
  }

  /// ### Params:
  /// * `result_tex` -  write
  /// * `color_source_tex` -  read
  /// * `linear_depth_tex` -  read
  fn execute_blur_single_direction(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut BlurFramebuffer,
    size: vk::Extent2D,
    result_tex: &mut VkTexture,       // write
    color_source_tex: &mut VkTexture, // read
    linear_depth_tex: &mut VkTexture, // read
    params: &BlurPassPushConstants,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        result_tex,
        color_source_tex,
        linear_depth_tex,
      );

      // start render pass
      cmd_begin_render_pass_for_framebuffer(
        &device,
        &command_buffer,
        &self.render_pass,
        &framebuffer.fbo,
        &size,
        &[],
      );
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // bind uniforms (do not move this)
      self.bind_uniforms(exec_ctx, params, color_source_tex, linear_depth_tex);

      // draw calls
      cmd_draw_fullscreen_triangle(device, &command_buffer);

      // end
      device.cmd_end_render_pass(command_buffer)
    }
  }

  unsafe fn bind_uniforms(
    &self,
    exec_ctx: &PassExecContext,
    push_constants: &BlurPassPushConstants,
    color_source_tex: &mut VkTexture,
    linear_depth_tex: &mut VkTexture,
  ) {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);

    let uniform_resouces = [
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
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
        binding: BINDING_INDEX_LINEAR_DEPTH,
        texture: linear_depth_tex,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
    ];
    bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);

    // push constants
    let push_constants_bytes = bytemuck::bytes_of(push_constants);
    device.cmd_push_constants(
      command_buffer,
      self.pipeline_layout,
      vk::ShaderStageFlags::FRAGMENT,
      0,
      push_constants_bytes,
    );
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    result_tex: &mut VkTexture,       // write
    color_source_tex: &mut VkTexture, // read
    linear_depth_tex: &mut VkTexture, // read
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

    let result_barrier = result_tex.barrier_prepare_attachment_for_write(); // we use stencil
    device.cmd_pipeline_barrier(
      *command_buffer,
      // wait for previous use in:
      vk::PipelineStageFlags::FRAGMENT_SHADER,
      // before we: write output
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[result_barrier],
    );
  }

  /// ### Params:
  /// * `color_source_tex` - 1st read, 2nd write
  /// * `tmp_ping_pong_tex` - 1st write, 2nd read
  /// * `linear_depth_tex` - read only
  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer0: &mut BlurFramebuffer,
    framebuffer1: &mut BlurFramebuffer,
    color_source_tex: &mut VkTexture,
    tmp_ping_pong_tex: &mut VkTexture,
    size: vk::Extent2D,
    linear_depth_tex: &mut VkTexture,
    u_blur_radius: usize,
    u_depth_max_dist: f32,
    u_gauss_sigma: f32,
  ) -> () {
    self.execute_blur_single_direction(
      exec_ctx,
      framebuffer0,
      size,
      tmp_ping_pong_tex,
      color_source_tex,
      linear_depth_tex,
      &BlurPassPushConstants {
        blur_direction: Self::BLUR_DIRECTION_PASS0,
        u_blur_radius: u_blur_radius as _,
        u_depth_max_dist,
        u_gauss_sigma,
      },
    );
    self.execute_blur_single_direction(
      exec_ctx,
      framebuffer1,
      size,
      color_source_tex,
      tmp_ping_pong_tex,
      linear_depth_tex,
      &BlurPassPushConstants {
        blur_direction: Self::BLUR_DIRECTION_PASS1,
        u_blur_radius: u_blur_radius as _,
        u_depth_max_dist,
        u_gauss_sigma,
      },
    );
  }
}

pub struct BlurFramebuffer {
  pub fbo: vk::Framebuffer,
}

impl BlurFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    device.destroy_framebuffer(self.fbo, None);
  }
}

#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
struct BlurPassPushConstants {
  pub blur_direction: Vec2,
  pub u_blur_radius: f32,
  pub u_depth_max_dist: f32,
  pub u_gauss_sigma: f32,
}

unsafe impl bytemuck::Zeroable for BlurPassPushConstants {}
unsafe impl bytemuck::Pod for BlurPassPushConstants {}
