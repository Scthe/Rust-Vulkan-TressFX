use ash;
use ash::vk;
use log::info;

use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_PREVIOUS_RESULT: u32 = 1;

const RESULT_TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UINT;
const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreen_quad.vert.spv",
  "./assets/shaders-compiled/tonemapping.frag.spv",
);

/// Does:
/// - dithering
/// - color grading
/// - outputs luma in alpha channel (later used for e.g. FXAA contrast detection)
pub struct TonemappingPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl TonemappingPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating {}", get_simple_type_name::<Self>());
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
      create_texture_binding(
        BINDING_INDEX_PREVIOUS_RESULT,
        vk::ShaderStageFlags::FRAGMENT,
      ),
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
  ) -> TonemappingPassFramebuffer {
    let device = vk_app.vk_device();

    let tonemapped_tex =
      vk_app.create_attachment::<Self>("tonemapped", frame_id, RESULT_TEXTURE_FORMAT, *size);

    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[tonemapped_tex.image_view()],
      &size,
    );

    TonemappingPassFramebuffer {
      tonemapped_tex,
      fbo,
    }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut TonemappingPassFramebuffer,
    previous_result: &mut VkTexture,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    let size = exec_ctx.size;
    let pass_name = &get_simple_type_name::<Self>();

    unsafe {
      self.cmd_resource_barriers(device, &command_buffer, framebuffer, previous_result);

      // start render pass
      let scope_id =
        exec_ctx.cmd_start_render_pass(pass_name, &self.render_pass, &framebuffer.fbo, &size, &[]);
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // bind uniforms (do not move this)
      self.bind_uniforms(exec_ctx, previous_result);

      // draw calls
      cmd_draw_fullscreen_triangle(device, &command_buffer);

      // end
      exec_ctx.cmd_end_render_pass(scope_id);
    }
  }

  unsafe fn bind_uniforms(&self, exec_ctx: &PassExecContext, previous_result: &mut VkTexture) {
    let vk_app = exec_ctx.vk_app;
    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);

    let uniform_resouces = [
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: BINDING_INDEX_CONFIG_UBO,
        buffer: exec_ctx.config_buffer,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_PREVIOUS_RESULT,
        texture: previous_result,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
    ];
    bind_resources_to_descriptors_graphic(&resouce_binder, 0, &uniform_resouces);
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    framebuffer: &mut TonemappingPassFramebuffer,
    previous_result: &mut VkTexture,
  ) {
    VkTexture::cmd_transition_attachments_for_read_barrier(
      device,
      *command_buffer,
      &mut [previous_result],
    );

    VkTexture::cmd_transition_attachments_for_write_barrier(
      device,
      *command_buffer,
      &mut [&mut framebuffer.tonemapped_tex],
    );
  }
}

pub struct TonemappingPassFramebuffer {
  pub tonemapped_tex: VkTexture,
  pub fbo: vk::Framebuffer,
}

impl TonemappingPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    self.tonemapped_tex.delete(device, allocator);
  }
}
