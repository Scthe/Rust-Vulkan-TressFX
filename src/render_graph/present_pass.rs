use ash;
use ash::vk;
use log::info;

use crate::app_ui::AppUI;
use crate::config::Config;
use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;
use crate::{either, vk_utils::*};

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_TONEMAPPED_RESULT: u32 = 1;
const BINDING_INDEX_NORMALS: u32 = 2;
const BINDING_INDEX_SSAO: u32 = 3;
const BINDING_INDEX_DEPTH: u32 = 4;
const BINDING_INDEX_SHADOW_MAP: u32 = 5;
const BINDING_INDEX_FORWARD_PASS_RESULT: u32 = 6;
const BINDING_INDEX_LINEAR_DEPTH: u32 = 7;

const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreen_quad.vert.spv",
  "./assets/shaders-compiled/present.frag.spv",
);

pub struct PresentPass {
  pub render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

/// Render to OS window framebuffer. Handles debug modes (e.g. shadow factor, normals).
/// Can shows debug positions of lights, shadow and SSS sources.
impl PresentPass {
  pub fn new(vk_app: &VkCtx, image_format: vk::Format) -> Self {
    info!("Creating {}", get_simple_type_name::<Self>());
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = Self::create_render_pass(device, image_format);
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

  fn create_render_pass(device: &ash::Device, image_format: vk::Format) -> vk::RenderPass {
    let load_op = either!(
      Config::TEST_ALPHA_COMPOSITE,
      vk::AttachmentLoadOp::CLEAR,
      vk::AttachmentLoadOp::DONT_CARE
    );
    let color_attachment = create_color_attachment(
      0,
      image_format,
      load_op, // we override every pixel regardless
      vk::AttachmentStoreOp::STORE,
      true,
    );

    unsafe { create_render_pass_from_attachments(device, None, &[color_attachment]) }
  }

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(BINDING_INDEX_CONFIG_UBO, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(
        BINDING_INDEX_TONEMAPPED_RESULT,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(BINDING_INDEX_NORMALS, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_SSAO, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_DEPTH, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_SHADOW_MAP, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(
        BINDING_INDEX_FORWARD_PASS_RESULT,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(BINDING_INDEX_LINEAR_DEPTH, vk::ShaderStageFlags::FRAGMENT),
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
    app_ui: &mut AppUI,
    forward_pass_result: &mut VkTexture,
    tonemapped_result: &mut VkTexture,
    normals_texture: &mut VkTexture,
    ssao_texture: &mut VkTexture,
    depth_stencil_tex: &mut VkTexture,
    depth_tex_image_view: vk::ImageView,
    shadow_map_texture: &mut VkTexture,
    linear_depth_texture: &mut VkTexture,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    let size = exec_ctx.size;
    let clear_values = exec_ctx.config.borrow().clear_swapchain_color();
    let pass_name = &get_simple_type_name::<Self>();

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        forward_pass_result,
        tonemapped_result,
        normals_texture,
        ssao_texture,
        depth_stencil_tex,
        shadow_map_texture,
        linear_depth_texture,
      );

      // start render pass
      let scope_id = exec_ctx.cmd_begin_scope(pass_name);
      exec_ctx.cmd_start_render_pass(
        &self.render_pass,
        &self.pipeline,
        &framebuffer,
        &size,
        &[clear_values],
      );

      // bind uniforms (do not move this)
      self.bind_uniforms(
        exec_ctx,
        forward_pass_result,
        tonemapped_result,
        normals_texture,
        ssao_texture,
        depth_stencil_tex,
        depth_tex_image_view,
        shadow_map_texture,
        linear_depth_texture,
      );

      // draw calls
      cmd_draw_fullscreen_triangle(device, &command_buffer);

      // ui
      app_ui.render_ui(exec_ctx, command_buffer);

      // end
      exec_ctx.cmd_end_render_pass(scope_id);
    }
  }

  unsafe fn bind_uniforms(
    &self,
    exec_ctx: &PassExecContext,
    forward_pass_result: &mut VkTexture,
    tonemapped_result: &mut VkTexture,
    normals_texture: &mut VkTexture,
    ssao_texture: &mut VkTexture,
    depth_stencil_tex: &mut VkTexture,
    depth_tex_image_view: vk::ImageView,
    shadow_map_texture: &mut VkTexture,
    linear_depth_texture: &mut VkTexture,
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
        binding: BINDING_INDEX_TONEMAPPED_RESULT,
        texture: tonemapped_result,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_NORMALS,
        texture: &normals_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SSAO,
        texture: &ssao_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_DEPTH,
        texture: &depth_stencil_tex,
        image_view: Some(depth_tex_image_view),
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SHADOW_MAP,
        texture: &shadow_map_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_FORWARD_PASS_RESULT,
        texture: &forward_pass_result,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_LINEAR_DEPTH,
        texture: &linear_depth_texture,
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
    forward_pass_result: &mut VkTexture,
    tonemapped_result: &mut VkTexture,
    normals_texture: &mut VkTexture,
    ssao_texture: &mut VkTexture,
    depth_texture: &mut VkTexture,
    shadow_map_texture: &mut VkTexture,
    linear_depth_texture: &mut VkTexture,
  ) {
    VkTexture::cmd_transition_attachments_for_read_barrier(
      device,
      *command_buffer,
      &mut [
        forward_pass_result,
        tonemapped_result,
        normals_texture,
        ssao_texture,
        depth_texture,
        shadow_map_texture,
        linear_depth_texture,
      ],
    );
  }
}
