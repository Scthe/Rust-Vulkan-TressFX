use ash;
use ash::vk;
use log::info;

use crate::config::Config;
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
const BINDING_INDEX_SHADOW_MAP: u32 = 5;
const BINDING_INDEX_SSS_DEPTH_MAP: u32 = 6;
const BINDING_INDEX_AO_TEX: u32 = 7;

const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/forward.vert.spv",
  "./assets/shaders-compiled/forward.frag.spv",
);

// TODO [LOW] ATM attachment data is split into create_framebuffer, render_pass, execute (cause clear color). Unify.
//      Or create RenderPass abstract class that will get some attachment desc and calc most of things

/// Render scene objects (not hair). Outputs `diffuse.rgb`, `normal.rgb` (packed) and `depth/stencil`.
/// Sets `SKIN` stencil flag.
///
/// Output is different for some special debug modes. E.g. shadow debug mode outputs shadow
/// preview - exact same values that are used in normal rendering path.
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
  pub const DEPTH_TEXTURE_FORMAT: vk::Format = vk::Format::D24_UNORM_S8_UINT;
  pub const DIFFUSE_TEXTURE_FORMAT: vk::Format = vk::Format::R32G32B32A32_SFLOAT;
  pub const NORMALS_TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_UINT;
  pub const COLOR_ATTACHMENT_COUNT: usize = 2;

  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating ForwardPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = Self::create_render_pass(device, vk::AttachmentLoadOp::CLEAR);
    let uniforms_desc = Self::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline = Self::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    let dummy_data_texture = Self::create_dummy_texture(vk_app);

    Self {
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

  /// Define render pass to compile shader against
  pub fn create_render_pass(device: &ash::Device, load_op: vk::AttachmentLoadOp) -> vk::RenderPass {
    // TODO [LOW] check if render pass can auto convert attachment layouts after execution? The `final_layout` param
    let depth_attachment = create_depth_stencil_attachment(
      0,
      Self::DEPTH_TEXTURE_FORMAT,
      load_op,                      // depth_load_op
      vk::AttachmentStoreOp::STORE, // depth_store_op
      load_op,                      // stencil_load_op
      vk::AttachmentStoreOp::STORE, // stencil_store_op
      vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    );
    let color_attachment = create_color_attachment(
      1,
      Self::DIFFUSE_TEXTURE_FORMAT,
      load_op,
      vk::AttachmentStoreOp::STORE,
      false,
    );
    let normals_attachment = create_color_attachment(
      2,
      Self::NORMALS_TEXTURE_FORMAT,
      load_op,
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
      create_texture_binding(BINDING_INDEX_SHADOW_MAP, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_SSS_DEPTH_MAP, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_AO_TEX, vk::ShaderStageFlags::FRAGMENT),
    ]
  }

  fn create_pipeline(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    pipeline_layout: &vk::PipelineLayout,
  ) -> vk::Pipeline {
    let vertex_desc = RenderableVertex::get_vertex_description();

    create_pipeline_with_defaults(
      device,
      render_pass,
      pipeline_layout,
      SHADER_PATHS,
      vertex_desc,
      Self::COLOR_ATTACHMENT_COUNT,
      |builder| {
        // TODO [MEDIUM] cull backfaces
        let stencil_write_skin = ps_stencil_write_if_touched(Config::STENCIL_BIT_SKIN, true);
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
          .depth_test_enable(true)
          .depth_write_enable(true)
          .depth_compare_op(vk::CompareOp::LESS)
          .depth_bounds_test_enable(false)
          .stencil_test_enable(true)
          .front(stencil_write_skin)
          .back(stencil_write_skin)
          .build();

        let pipeline_create_info = builder.depth_stencil_state(&depth_stencil).build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  /// Separate fn as some other passess will use same parameters (e.g. sss blur for ping-pong).
  pub fn create_diffuse_attachment_tex(
    vk_app: &VkCtx,
    size: &vk::Extent2D,
    name: String,
  ) -> VkTexture {
    vk_app.create_texture_empty(
      name,
      *size,
      Self::DIFFUSE_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::MemoryPropertyFlags::DEVICE_LOCAL,
      vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    frame_id: usize,
    size: &vk::Extent2D,
  ) -> ForwardPassFramebuffer {
    let device = vk_app.vk_device();

    let depth_stencil_tex = vk_app.create_texture_empty(
      format!("ForwardPass.depth#{}", frame_id),
      *size,
      Self::DEPTH_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::MemoryPropertyFlags::DEVICE_LOCAL,
      vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
    );
    let diffuse_tex = Self::create_diffuse_attachment_tex(
      vk_app,
      size,
      format!("ForwardPass.diffuse#{}", frame_id),
    );

    let normals_tex = vk_app.create_texture_empty(
      format!("ForwardPass.normal#{}", frame_id),
      *size,
      Self::NORMALS_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::MemoryPropertyFlags::DEVICE_LOCAL,
      vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    );

    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[
        depth_stencil_tex.image_view(),
        diffuse_tex.image_view(),
        normals_tex.image_view(),
      ],
      &size,
    );

    let depth_image_view =
      depth_stencil_tex.create_extra_image_view(device, vk::ImageAspectFlags::DEPTH);

    ForwardPassFramebuffer {
      depth_stencil_tex,
      depth_image_view,
      diffuse_tex,
      normals_tex,
      fbo,
    }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut ForwardPassFramebuffer,
    shadow_map_texture: &mut VkTexture,
    sss_depth_texture: &mut VkTexture,
    ao_texture: &mut VkTexture,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let scene = &*exec_ctx.scene;
    let config = &exec_ctx.config;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    let clear_values = [
      config.clear_depth_stencil(),
      config.clear_color(),
      config.clear_normals(),
    ];

    // TODO [LOW] no need to rerecord every frame TBH. Everything can be controlled by uniforms etc.
    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        framebuffer,
        shadow_map_texture,
        sss_depth_texture,
        ao_texture,
      );

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
        self.bind_entity_ubos(
          exec_ctx,
          shadow_map_texture,
          sss_depth_texture,
          ao_texture,
          entity,
        );
        entity.cmd_bind_mesh_buffers(device, command_buffer);
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
    sss_depth_texture: &mut VkTexture,
    ao_texture: &mut VkTexture,
  ) {
    let shadow_map_barrier = shadow_map_texture.barrier_prepare_attachment_for_shader_read();
    let sss_depth_barrier = sss_depth_texture.barrier_prepare_attachment_for_shader_read();
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
      &[shadow_map_barrier, sss_depth_barrier, ao_barrier],
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
      vk::PipelineStageFlags::FRAGMENT_SHADER,
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
    shadow_map_texture: &mut VkTexture,
    sss_depth_texture: &mut VkTexture,
    ao_texture: &mut VkTexture,
    entity: &WorldEntity,
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
        usage: BindableBufferUsage::UBO,
        binding: BINDING_INDEX_MODEL_UBO,
        buffer: entity.get_ubo_buffer(frame_id),
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_DIFFUSE_TEXTURE,
        texture: &entity.material.albedo_tex,
        image_view: None,
        sampler: vk_app.default_texture_sampler_linear,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SPECULAR_TEXTURE,
        texture: &entity
          .material
          .specular_tex
          .as_ref()
          .unwrap_or(&self.dummy_data_texture),
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_HAIR_SHADOW_TEXTURE,
        texture: &entity
          .material
          .hair_shadow_tex
          .as_ref()
          .unwrap_or(&self.dummy_data_texture),
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SHADOW_MAP,
        texture: &shadow_map_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_SSS_DEPTH_MAP,
        texture: &sss_depth_texture,
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

  fn create_dummy_texture(vk_app: &VkCtx) -> VkTexture {
    vk_app.create_texture_empty(
      "ForwardPassDummyDataTex".to_string(),
      vk::Extent2D {
        width: 4,
        height: 4,
      },
      VkTexture::RAW_DATA_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::SAMPLED,
      vk::MemoryPropertyFlags::DEVICE_LOCAL,
      vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )
  }
}

pub struct ForwardPassFramebuffer {
  pub depth_stencil_tex: VkTexture,
  /// Used to read only depth from `depth_stencil_tex`
  pub depth_image_view: vk::ImageView,
  pub diffuse_tex: VkTexture,
  pub normals_tex: VkTexture,
  pub fbo: vk::Framebuffer,
}

impl ForwardPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    device.destroy_image_view(self.depth_image_view, None);
    self.depth_stencil_tex.delete(device, allocator);
    self.diffuse_tex.delete(device, allocator);
    self.normals_tex.delete(device, allocator);
  }
}
