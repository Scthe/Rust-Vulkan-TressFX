use ash;
use ash::vk;
use log::info;

use crate::config::Config;
use crate::render_graph::forward_pass::ForwardPass;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use crate::render_graph::PassExecContext;

const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreen_quad.vert.spv",
  "./assets/shaders-compiled/tfx_ppll_resolve.frag.spv",
);

/// https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L610
pub struct TfxPpllResolvePass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl TfxPpllResolvePass {
  const COLOR_ATTACHMENT_COUNT: usize = 1;

  const BINDING_INDEX_CONFIG_UBO: u32 = 0;
  const BINDING_INDEX_HEAD_POINTERS_IMAGE: u32 = 1; // Must match shader
  const BINDING_INDEX_DATA_BUFFER: u32 = 2; // Must match shader

  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating TfxPpllResolvePass");
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

  pub unsafe fn destroy(&self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
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

    unsafe {
      create_render_pass_from_attachments(device, Some(depth_attachment), &[color_attachment])
    }
  }

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(
        Self::BINDING_INDEX_CONFIG_UBO,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_storage_image_binding(
        Self::BINDING_INDEX_HEAD_POINTERS_IMAGE,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_DATA_BUFFER,
        vk::ShaderStageFlags::FRAGMENT,
      ),
    ]
  }

  /// https://github.com/Scthe/TressFX-OpenGL/blob/master/src/gl-tfx/TFxPPLL.cpp#L35
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
      Self::COLOR_ATTACHMENT_COUNT,
      |builder| {
        // depth (ignored) + stencil (only hair)
        let stencil_only_hair = ps_stencil_compare_equal(Config::STENCIL_BIT_HAIR);
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
          .depth_test_enable(false)
          .depth_write_enable(false)
          .depth_compare_op(vk::CompareOp::ALWAYS)
          .depth_bounds_test_enable(false)
          .stencil_test_enable(true)
          .front(stencil_only_hair)
          .back(stencil_only_hair)
          .build();

        let blend_hair_color_attachment = vk::PipelineColorBlendAttachmentState::builder()
          .color_write_mask(vk::ColorComponentFlags::RGBA)
          .blend_enable(true)
          .color_blend_op(vk::BlendOp::ADD)
          .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA) // shader output
          .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA) // existing value on destination attachment
          .alpha_blend_op(vk::BlendOp::ADD)
          .src_alpha_blend_factor(vk::BlendFactor::ZERO) // shader output
          .dst_alpha_blend_factor(vk::BlendFactor::ONE) // existing value on destination attachment
          .build();
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
          .attachments(&[blend_hair_color_attachment])
          .build();

        // finish
        let pipeline_create_info = builder
          .depth_stencil_state(&depth_stencil)
          .color_blend_state(&color_blend_state)
          .build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    depth_stencil_tex: &VkTexture,
    forward_color_tex: &VkTexture,
  ) -> TfxPpllResolvePassFramebuffer {
    let device = vk_app.vk_device();

    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[
        depth_stencil_tex.image_view(),
        forward_color_tex.image_view(),
      ],
      &depth_stencil_tex.size(),
    );

    TfxPpllResolvePassFramebuffer { fbo }
  }

  /// https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L610
  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut TfxPpllResolvePassFramebuffer,
    depth_stencil_tex: &mut VkTexture,
    forward_color_tex: &mut VkTexture,
    ppll_head_pointers_image: &mut VkTexture,
    ppll_data_buffer: &mut VkBuffer,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    if !exec_ctx.scene.has_hair_objects() {
      return;
    }

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        depth_stencil_tex,
        forward_color_tex,
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

      // bind uniforms (do not move this)
      self.bind_uniforms(exec_ctx, ppll_head_pointers_image, ppll_data_buffer);

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
    depth_stencil_tex: &mut VkTexture,
    forward_color_tex: &mut VkTexture,
  ) {
    execute_full_pipeline_barrier(device, *command_buffer); // TODO [PPLL_sync] remove

    // Make a pipeline barrier to guarantee the geometry pass is done
    // https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L610
    // TODO [PPLL_sync] seems suboptimal. Why not provide ppll resources for previous pass?
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
      &[],
    );

    VkTexture::cmd_transition_attachments_for_write_barrier(
      device,
      *command_buffer,
      &mut [depth_stencil_tex, forward_color_tex],
    );
  }

  unsafe fn bind_uniforms(
    &self,
    exec_ctx: &PassExecContext,
    ppll_head_pointers_image: &mut VkTexture,
    ppll_data_buffer: &mut VkBuffer,
  ) {
    let vk_app = exec_ctx.vk_app;
    let config_buffer = exec_ctx.config_buffer;
    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);

    let uniform_resouces = [
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: Self::BINDING_INDEX_CONFIG_UBO,
        buffer: config_buffer,
      },
      BindableResource::StorageImage {
        binding: Self::BINDING_INDEX_HEAD_POINTERS_IMAGE,
        texture: &ppll_head_pointers_image,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_DATA_BUFFER,
        buffer: &ppll_data_buffer,
      },
    ];
    bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);
  }
}

pub struct TfxPpllResolvePassFramebuffer {
  pub fbo: vk::Framebuffer,
}

impl TfxPpllResolvePassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    device.destroy_framebuffer(self.fbo, None);
  }
}
