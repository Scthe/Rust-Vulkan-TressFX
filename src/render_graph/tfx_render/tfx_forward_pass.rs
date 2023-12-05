use ash;
use ash::vk;
use log::info;

use crate::scene::TfxObject;
use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use crate::render_graph::forward_pass::{ForwardPass, ForwardPassFramebuffer};
use crate::render_graph::PassExecContext;

const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/tfx_forward.vert.spv",
  "./assets/shaders-compiled/tfx_forward.frag.spv",
);

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
  pub const BINDING_INDEX_CONFIG_UBO: u32 = 0;
  pub const BINDING_INDEX_POSITIONS_SSBO: u32 = 1;
  pub const BINDING_INDEX_TANGENTS_SSBO: u32 = 2;
  pub const BINDING_INDEX_TFX_PARAMS_UBO: u32 = 3;
  pub const BINDING_INDEX_SHADOW_MAP: u32 = 4;
  pub const BINDING_INDEX_AO_TEX: u32 = 5;

  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating {}", get_simple_type_name::<Self>());
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = ForwardPass::create_render_pass(device, vk::AttachmentLoadOp::LOAD);
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

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(
        Self::BINDING_INDEX_CONFIG_UBO,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_POSITIONS_SSBO,
        vk::ShaderStageFlags::VERTEX,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_TANGENTS_SSBO,
        vk::ShaderStageFlags::VERTEX,
      ),
      create_ubo_binding(
        Self::BINDING_INDEX_TFX_PARAMS_UBO,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(
        Self::BINDING_INDEX_SHADOW_MAP,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(Self::BINDING_INDEX_AO_TEX, vk::ShaderStageFlags::FRAGMENT),
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
        // TODO [IGNORE] write hair stencil bit
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
    let command_buffer = exec_ctx.command_buffer;
    let size = exec_ctx.size;
    let device = vk_app.vk_device();
    let pass_name = &get_simple_type_name::<Self>();

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        framebuffer,
        shadow_map_texture,
        ao_texture,
      );

      // start render pass
      let scope_id = exec_ctx.cmd_begin_scope(pass_name);
      exec_ctx.cmd_start_render_pass(
        &self.render_pass,
        &self.pipeline,
        &framebuffer.fbo,
        &size,
        &[],
      );

      // draw calls
      let scene = exec_ctx.scene.borrow();
      for entity in &scene.tressfx_objects {
        self.bind_entity_ubos(exec_ctx, entity, shadow_map_texture, ao_texture);
        entity.cmd_draw_mesh(device, command_buffer);
      }

      // end
      exec_ctx.cmd_end_render_pass(scope_id);
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
    VkTexture::cmd_transition_attachments_for_read_barrier(
      device,
      *command_buffer,
      &mut [shadow_map_texture, ao_texture],
    );

    VkTexture::cmd_transition_attachments_for_write_barrier(
      device,
      *command_buffer,
      &mut [
        &mut framebuffer.diffuse_tex,
        &mut framebuffer.normals_tex,
        &mut framebuffer.depth_stencil_tex,
      ],
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

    let uniform_resouces = [
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: Self::BINDING_INDEX_CONFIG_UBO,
        buffer: config_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_POSITIONS_SSBO,
        buffer: &entity.get_current_position_buffer(exec_ctx.timer.frame_idx()),
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_TANGENTS_SSBO,
        buffer: &entity.tangents_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: Self::BINDING_INDEX_TFX_PARAMS_UBO,
        buffer: &entity.get_tfx_params_ubo_buffer(exec_ctx.frame_in_flight_id),
      },
      BindableResource::Texture {
        binding: Self::BINDING_INDEX_SHADOW_MAP,
        texture: &shadow_map_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: Self::BINDING_INDEX_AO_TEX,
        texture: &ao_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_linear,
      },
    ];

    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
    bind_resources_to_descriptors_graphic(&resouce_binder, 0, &uniform_resouces);
  }
}
