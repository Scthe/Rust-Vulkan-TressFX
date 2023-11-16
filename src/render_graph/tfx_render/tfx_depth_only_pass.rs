use ash;
use ash::vk;
use log::info;

use crate::render_graph::forward_pass::ForwardPass;
use crate::render_graph::tfx_render::TfxForwardPass;
use crate::scene::TfxObject;
use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const COLOR_ATTACHMENT_COUNT: usize = 0;
const SHADER_PATHS: (&str, &str) = (
  // Entire new vertex shader because Vulkan complains other passes vertex shader outputs are not used.
  "./assets/shaders-compiled/tfx_depth_only.vert.spv",
  "./assets/shaders-compiled/shadow_map_gen.frag.spv",
);

/// Same as `ShadowMapPass`, but hair only and preserves current depth value.
pub struct TfxDepthOnlyPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl TfxDepthOnlyPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating TfxDepthOnlyPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = Self::create_render_pass(device);
    let uniforms_desc = Self::get_uniforms_layout_hair();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline =
      Self::create_pipeline_tressfx_hair(device, pipeline_cache, &render_pass, &pipeline_layout);

    Self {
      render_pass,
      pipeline,
      pipeline_layout,
      uniforms_layout,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
    device.destroy_descriptor_set_layout(self.uniforms_layout, None);
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

    unsafe { create_render_pass_from_attachments(device, Some(depth_attachment), &[]) }
  }

  fn get_uniforms_layout_hair() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(
        TfxForwardPass::BINDING_INDEX_CONFIG_UBO,
        vk::ShaderStageFlags::VERTEX,
      ),
      create_ssbo_binding(
        TfxForwardPass::BINDING_INDEX_POSITIONS_SSBO,
        vk::ShaderStageFlags::VERTEX,
      ),
      create_ssbo_binding(
        TfxForwardPass::BINDING_INDEX_TANGENTS_SSBO,
        vk::ShaderStageFlags::VERTEX,
      ),
      create_ubo_binding(
        TfxForwardPass::BINDING_INDEX_TFX_PARAMS_UBO,
        vk::ShaderStageFlags::VERTEX,
      ),
    ]
  }

  fn create_pipeline_tressfx_hair(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    pipeline_layout: &vk::PipelineLayout,
  ) -> vk::Pipeline {
    let vertex_desc = ps_vertex_empty();
    Self::create_pipeline(
      device,
      pipeline_cache,
      render_pass,
      pipeline_layout,
      SHADER_PATHS,
      vertex_desc,
    )
  }

  fn create_pipeline(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    pipeline_layout: &vk::PipelineLayout,
    shader_paths: (&str, &str),
    vertex_desc: vk::PipelineVertexInputStateCreateInfo,
  ) -> vk::Pipeline {
    create_pipeline_with_defaults(
      device,
      render_pass,
      pipeline_layout,
      shader_paths,
      vertex_desc,
      COLOR_ATTACHMENT_COUNT,
      |builder| {
        let pipeline_create_info = builder
          .depth_stencil_state(&ps_depth_less_stencil_always())
          // see https://docs.microsoft.com/en-us/windows/desktop/DxTechArts/common-techniques-to-improve-shadow-depth-maps#back-face-and-front-face
          .rasterization_state(&ps_raster_polygons(vk::CullModeFlags::NONE))
          .build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  pub fn create_framebuffer(&self, vk_app: &VkCtx, depth_tex: &VkTexture) -> vk::Framebuffer {
    let device = vk_app.vk_device();
    create_framebuffer(
      device,
      self.render_pass,
      &[depth_tex.image_view()],
      &depth_tex.size(),
    )
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    fbo: vk::Framebuffer,
    depth_tex: &mut VkTexture,
    entity: &TfxObject,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    let pass_type_name = get_simple_type_name::<Self>();
    let pass_name = format!("{}.{}", pass_type_name, entity.name);

    unsafe {
      self.cmd_resource_barriers(device, &command_buffer, depth_tex);

      // start render pass
      let scope_id =
        exec_ctx.cmd_start_render_pass(&pass_name, &self.render_pass, &fbo, &depth_tex.size(), &[]);

      // draw hair
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );
      self.bind_hair_ubos(exec_ctx, entity);
      entity.cmd_draw_mesh(device, command_buffer);

      // end
      exec_ctx.cmd_end_render_pass(scope_id);
    }
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    depth_tex: &mut VkTexture,
  ) {
    VkTexture::cmd_transition_attachments_for_write_barrier(
      device,
      *command_buffer,
      &mut [depth_tex],
    );
  }

  unsafe fn bind_hair_ubos(&self, exec_ctx: &PassExecContext, entity: &TfxObject) {
    let config_buffer = exec_ctx.config_buffer;
    let frame_id = exec_ctx.swapchain_image_idx;

    let uniform_resouces = [
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: TfxForwardPass::BINDING_INDEX_CONFIG_UBO,
        buffer: config_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: TfxForwardPass::BINDING_INDEX_POSITIONS_SSBO,
        buffer: &entity.get_current_position_buffer(exec_ctx.timer.frame_idx()),
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: TfxForwardPass::BINDING_INDEX_TANGENTS_SSBO,
        buffer: &entity.tangents_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: TfxForwardPass::BINDING_INDEX_TFX_PARAMS_UBO,
        buffer: &entity.get_tfx_params_ubo_buffer(frame_id),
      },
    ];

    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
    bind_resources_to_descriptors_graphic(&resouce_binder, 0, &uniform_resouces);
  }
}
