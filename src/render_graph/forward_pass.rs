use ash;
use ash::vk;
use log::info;

use crate::render_graph::_shared::RenderableVertex;
use crate::scene::WorldEntity;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_MODEL_UBO: u32 = 1;
const BINDING_INDEX_DIFFUSE_TEXTURE: u32 = 2;
const BINDING_INDEX_SPECULAR_TEXTURE: u32 = 3;
const BINDING_INDEX_HAIR_SHADOW_TEXTURE: u32 = 4;

const DEPTH_TEXTURE_FORMAT: vk::Format = vk::Format::D24_UNORM_S8_UINT;
const DIFFUSE_TEXTURE_FORMAT: vk::Format = vk::Format::R32G32B32A32_SFLOAT;
const NORMALS_TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UINT; // TODO try float?
const COLOR_ATTACHMENT_COUNT: usize = 2;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/forward.vert.spv",
  "./assets/shaders-compiled/forward.frag.spv",
);

// TODO ATM attachment data is split into create_framebuffer, render_pass, execute (cause clear color). Unify.
//      Or create RenderPass abstract class that will get some attachment desc and calc most of things

pub struct ForwardPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
  /// When shader expects e.g. specular texture, but object does not have one.
  /// This texture is only for binding, please do not rely on any value inside.
  /// Has format `VkTexture::RAW_DATA_TEXTURE_FORMAT`.
  dummy_data_texture: VkTexture,
}

impl ForwardPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating ForwardPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = ForwardPass::create_render_pass(device);
    let uniforms_desc = ForwardPass::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline =
      ForwardPass::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    let dummy_data_texture = ForwardPass::create_dummy_texture(vk_app);

    ForwardPass {
      render_pass,
      pipeline,
      pipeline_layout,
      uniforms_layout,
      dummy_data_texture,
    }
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_descriptor_set_layout(self.uniforms_layout, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
    self.dummy_data_texture.delete(device, allocator);
  }

  fn create_render_pass(device: &ash::Device) -> vk::RenderPass {
    // TODO check if render pass can auto convert attachment layouts after execution? The `final_layout` param
    // 1. define render pass to compile shader against
    let depth_attachment = create_depth_stencil_attachment(
      0,
      DEPTH_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::CLEAR,      // depth_load_op
      vk::AttachmentStoreOp::STORE,     // depth_store_op
      vk::AttachmentLoadOp::DONT_CARE,  // stencil_load_op
      vk::AttachmentStoreOp::DONT_CARE, // stencil_store_op
      vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    );
    let color_attachment = create_color_attachment(
      1,
      DIFFUSE_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::CLEAR,
      vk::AttachmentStoreOp::STORE,
      false,
    );
    let normals_attachment = create_color_attachment(
      2,
      NORMALS_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::CLEAR,
      vk::AttachmentStoreOp::STORE,
      false,
    );

    let subpass = vk::SubpassDescription::builder()
      .color_attachments(&[color_attachment.1, normals_attachment.1])
      .depth_stencil_attachment(&depth_attachment.1)
      .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
      .build();

    // needed as we first clear the depth/color attachments in `vk::AttachmentLoadOp`
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
      .attachments(&[depth_attachment.0, color_attachment.0, normals_attachment.0])
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
      create_ubo_binding(
        BINDING_INDEX_CONFIG_UBO,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
      ),
      create_ubo_binding(
        BINDING_INDEX_MODEL_UBO,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(
        BINDING_INDEX_DIFFUSE_TEXTURE,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(
        BINDING_INDEX_SPECULAR_TEXTURE,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_texture_binding(
        BINDING_INDEX_HAIR_SHADOW_TEXTURE,
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
    let vertex_desc = vk::PipelineVertexInputStateCreateInfo::builder()
      .vertex_attribute_descriptions(&RenderableVertex::get_attributes_descriptions())
      .vertex_binding_descriptions(&RenderableVertex::get_bindings_descriptions())
      .build();

    create_pipeline_with_defaults(
      device,
      render_pass,
      pipeline_layout,
      SHADER_PATHS,
      vertex_desc,
      COLOR_ATTACHMENT_COUNT,
      |builder| {
        // TODO cull backfaces
        let pipeline_create_info = builder
          .depth_stencil_state(&ps_depth_less_stencil_always())
          .build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    frame_id: usize,
    size: &vk::Extent2D,
  ) -> ForwardPassFramebuffer {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    let depth_tex = VkTexture::empty(
      device,
      allocator,
      format!("ForwardPass.depth#{}", frame_id),
      *size,
      DEPTH_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::DEPTH,
    );
    let diffuse_tex = VkTexture::empty(
      device,
      allocator,
      format!("ForwardPass.diffuse#{}", frame_id),
      *size,
      DIFFUSE_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
    );
    let normals_tex = VkTexture::empty(
      device,
      allocator,
      format!("ForwardPass.normal#{}", frame_id),
      *size,
      NORMALS_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
    );

    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[
        depth_tex.image_view(),
        diffuse_tex.image_view(),
        normals_tex.image_view(),
      ],
      &size,
    );

    ForwardPassFramebuffer {
      depth_tex,
      diffuse_tex,
      normals_tex,
      fbo,
    }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut ForwardPassFramebuffer,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let scene = exec_ctx.scene;
    let config = &exec_ctx.config;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    let clear_values = [
      config.clear_depth_stencil(),
      config.clear_color(),
      config.clear_normals(),
    ];

    // TODO no need to rerecord every frame TBH. Everything can be controlled by uniforms etc.
    unsafe {
      self.cmd_resource_barriers(device, &command_buffer, framebuffer);

      // start render pass
      cmd_begin_render_pass_for_framebuffer(
        &device,
        &command_buffer,
        &self.render_pass,
        &framebuffer.fbo,
        &exec_ctx.size,
        &clear_values,
      );
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // draw calls
      for entity in &scene.entities {
        self.bind_entity_ubos(exec_ctx, entity);
        device.cmd_bind_vertex_buffers(command_buffer, 0, &[entity.vertex_buffer.buffer], &[0]);
        device.cmd_bind_index_buffer(
          command_buffer,
          entity.index_buffer.buffer,
          0,
          vk::IndexType::UINT32,
        );
        device.cmd_draw_indexed(command_buffer, entity.vertex_count, 1, 0, 0, 0);
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
  ) {
    let diffuse_barrier = framebuffer.diffuse_tex.prepare_for_layout_transition(
      vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
      vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
      vk::AccessFlags::SHADER_READ,
    );
    let normal_barrier = framebuffer.normals_tex.prepare_for_layout_transition(
      vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
      vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
      vk::AccessFlags::SHADER_READ,
    );
    device.cmd_pipeline_barrier(
      *command_buffer,
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      vk::PipelineStageFlags::FRAGMENT_SHADER,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[diffuse_barrier, normal_barrier],
    );
  }

  unsafe fn bind_entity_ubos(&self, exec_ctx: &PassExecContext, entity: &WorldEntity) {
    let vk_app = exec_ctx.vk_app;
    let config_buffer = exec_ctx.config_buffer;
    let frame_id = exec_ctx.swapchain_image_idx;

    let uniform_resouces = [
      BindableResource::Uniform {
        binding: BINDING_INDEX_CONFIG_UBO,
        buffer: config_buffer,
      },
      BindableResource::Uniform {
        binding: BINDING_INDEX_MODEL_UBO,
        buffer: entity.get_ubo_buffer(frame_id),
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_DIFFUSE_TEXTURE,
        texture: &entity.material.albedo_tex,
        sampler: vk_app.default_texture_sampler_linear,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SPECULAR_TEXTURE,
        texture: &entity
          .material
          .specular_tex
          .as_ref()
          .unwrap_or(&self.dummy_data_texture),
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_HAIR_SHADOW_TEXTURE,
        texture: &entity
          .material
          .hair_shadow_tex
          .as_ref()
          .unwrap_or(&self.dummy_data_texture),
        sampler: vk_app.default_texture_sampler_nearest,
      },
    ];

    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
    bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);
  }

  fn create_dummy_texture(vk_app: &VkCtx) -> VkTexture {
    let device = vk_app.vk_device();
    let mut dummy_data_texture = VkTexture::empty(
      device,
      &vk_app.allocator,
      "ForwardPassDummyDataTex".to_string(),
      vk::Extent2D {
        width: 4,
        height: 4,
      },
      VkTexture::RAW_DATA_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
    );
    dummy_data_texture.force_image_layout(vk_app, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

    dummy_data_texture
  }
}

pub struct ForwardPassFramebuffer {
  pub depth_tex: VkTexture,
  pub diffuse_tex: VkTexture,
  pub normals_tex: VkTexture,
  pub fbo: vk::Framebuffer,
}

impl ForwardPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    self.depth_tex.delete(device, allocator);
    self.diffuse_tex.delete(device, allocator);
    self.normals_tex.delete(device, allocator);
  }
}
