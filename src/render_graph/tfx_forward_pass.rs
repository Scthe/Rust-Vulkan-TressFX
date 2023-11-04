use ash;
use ash::vk;
use log::info;

use crate::scene::TfxObject;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::forward_pass::{ForwardPass, ForwardPassFramebuffer};
use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_POSITIONS_SSBO: u32 = 1;
const BINDING_INDEX_TANGENTS_SSBO: u32 = 2;
const BINDING_INDEX_TFX_PARAMS_UBO: u32 = 3;
const BINDING_INDEX_SHADOW_MAP: u32 = 4;
const BINDING_INDEX_AO_TEX: u32 = 5;

const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/tfx_forward.vert.spv",
  "./assets/shaders-compiled/tfx_forward.frag.spv",
);

// TODO tressfx pass:
// - remove hardcoded consts
// - add ui
// - finish rendering
// - add debug modes
// - rerender linear depth after hair
// - add hair shadows + ?ao?

/// Forward render TressFX hair asset. Same attachment textures as `ForwardPass`
/// (used with `AttachmentLoadOp::LOAD` to preserve the values). Sets `HAIR` stencil flag.
///
/// Has debug modes for hair, independent from the global debug previews.
pub struct TfxForwardPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl TfxForwardPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating TfxForwardPass");
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

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_descriptor_set_layout(self.uniforms_layout, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
  }

  // TODO copy-pasted from `ForwardPass`, but we do no longer `vk::AttachmentLoadOp::CLEAR`
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
    let normals_attachment = create_color_attachment(
      2,
      ForwardPass::NORMALS_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::LOAD,
      vk::AttachmentStoreOp::STORE,
      false,
    );

    unsafe {
      create_render_pass_from_attachments(
        device,
        Some(depth_attachment),
        &[color_attachment, normals_attachment],
      )
    }
  }

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(
        BINDING_INDEX_CONFIG_UBO,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
      ),
      create_ssbo_binding(BINDING_INDEX_POSITIONS_SSBO, vk::ShaderStageFlags::VERTEX),
      create_ssbo_binding(BINDING_INDEX_TANGENTS_SSBO, vk::ShaderStageFlags::VERTEX),
      create_ubo_binding(
        BINDING_INDEX_TFX_PARAMS_UBO,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(BINDING_INDEX_SHADOW_MAP, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_AO_TEX, vk::ShaderStageFlags::FRAGMENT),
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
      ForwardPass::COLOR_ATTACHMENT_COUNT,
      |builder| {
        // TODO write hair stencil bit
        let depth_stencil = ps_depth_less_stencil_always();

        let pipeline_create_info = builder.depth_stencil_state(&depth_stencil).build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut ForwardPassFramebuffer,
    shadow_map_texture: &mut VkTexture,
    ao_texture: &mut VkTexture,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let scene = &*exec_ctx.scene;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        framebuffer,
        shadow_map_texture,
        ao_texture,
      );

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

      // draw calls
      for entity in &scene.tressfx_objects {
        self.bind_entity_ubos(exec_ctx, entity, shadow_map_texture, ao_texture);
        entity.cmd_draw_mesh(device, command_buffer);
      }

      // end
      device.cmd_end_render_pass(command_buffer)
    }
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    framebuffer: &mut ForwardPassFramebuffer,
    shadow_map_texture: &mut VkTexture,
    ao_texture: &mut VkTexture,
  ) {
    let shadow_map_barrier = shadow_map_texture.barrier_prepare_attachment_for_shader_read();
    let ao_barrier = ao_texture.barrier_prepare_attachment_for_shader_read();

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
      &[shadow_map_barrier, ao_barrier],
    );

    let diffuse_barrier = framebuffer
      .diffuse_tex
      .barrier_prepare_attachment_for_write();
    let normal_barrier = framebuffer
      .normals_tex
      .barrier_prepare_attachment_for_write();
    let depth_barrier = framebuffer
      .depth_stencil_tex
      .barrier_prepare_attachment_for_write();

    device.cmd_pipeline_barrier(
      *command_buffer,
      // wait for previous use in:
      vk::PipelineStageFlags::FRAGMENT_SHADER
        | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
        | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      // before we: execute depth test or write output
      vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
        | vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[depth_barrier, diffuse_barrier, normal_barrier],
    );
  }

  unsafe fn bind_entity_ubos(
    &self,
    exec_ctx: &PassExecContext,
    entity: &TfxObject,
    shadow_map_texture: &mut VkTexture,
    ao_texture: &mut VkTexture,
  ) {
    let vk_app = exec_ctx.vk_app;
    let config_buffer = exec_ctx.config_buffer;
    let frame_id = exec_ctx.swapchain_image_idx;

    let uniform_resouces = [
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: BINDING_INDEX_CONFIG_UBO,
        buffer: config_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: BINDING_INDEX_POSITIONS_SSBO,
        buffer: &entity.positions_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: BINDING_INDEX_TANGENTS_SSBO,
        buffer: &entity.tangents_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: BINDING_INDEX_TFX_PARAMS_UBO,
        buffer: &entity.get_tfx_params_ubo_buffer(frame_id),
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SHADOW_MAP,
        texture: &shadow_map_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_AO_TEX,
        texture: &ao_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_linear,
      },
    ];

    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
    bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);
  }
}
