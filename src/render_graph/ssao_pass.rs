use ash;
use ash::vk;
use log::info;

use crate::config::SSAOConfig;
use crate::utils::{get_simple_type_name, RngVectorGenerator};
use crate::vk_ctx::VkCtx;
use crate::{either, vk_utils::*};

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_SCENE_DEPTH: u32 = 1;
const BINDING_INDEX_NORMAL: u32 = 2;
const BINDING_INDEX_NOISE: u32 = 3;
const BINDING_INDEX_KERNEL: u32 = 4;

const NOISE_TEXTURE_FORMAT: vk::Format = vk::Format::R32G32B32A32_SFLOAT;
const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreen_quad.vert.spv",
  "./assets/shaders-compiled/ssao.frag.spv",
);

/// Screen space ambient occlusion. Not much more to say about it.
/// Probably half screen size, but see `Config` to verify.
pub struct SSAOPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
  /// Random unit vectors used to create tangent-bitangent-normal coordinate system.
  rng_vectors_texture: VkTexture,
  /// Sampling vectors from currently shaded point.
  /// Not unit vectors, we want to sample not only the 'shell' of the hemisphere
  /// around us, but it's content.
  rng_sample_directions_kernel: VkBuffer,
}

impl SSAOPass {
  pub const RESULT_TEXTURE_FORMAT: vk::Format = vk::Format::R32_SFLOAT;

  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating {}", get_simple_type_name::<Self>());
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = Self::create_render_pass(device);
    let uniforms_desc = Self::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline = Self::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    let rng_vectors_texture =
      create_random_sampling_texture(vk_app, SSAOConfig::RNG_VECTOR_TEXTURE_SIZE);
    let rng_sample_directions_kernel =
      create_random_directions_kernel(vk_app, SSAOConfig::MAX_KERNEL_VALUES);

    Self {
      render_pass,
      pipeline,
      pipeline_layout,
      uniforms_layout,
      rng_vectors_texture,
      rng_sample_directions_kernel,
    }
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_descriptor_set_layout(self.uniforms_layout, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
    self.rng_vectors_texture.delete(device, allocator);
    self.rng_sample_directions_kernel.delete(allocator);
  }

  fn create_render_pass(device: &ash::Device) -> vk::RenderPass {
    let color_attachment = create_color_attachment(
      0,
      Self::RESULT_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::DONT_CARE, // we override every pixel regardless
      vk::AttachmentStoreOp::STORE,
    );

    unsafe { create_render_pass_from_attachments(device, None, &[color_attachment]) }
  }

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(BINDING_INDEX_CONFIG_UBO, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_SCENE_DEPTH, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_NORMAL, vk::ShaderStageFlags::FRAGMENT),
      create_texture_binding(BINDING_INDEX_NOISE, vk::ShaderStageFlags::FRAGMENT),
      create_ubo_binding(BINDING_INDEX_KERNEL, vk::ShaderStageFlags::FRAGMENT),
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

  pub fn create_result_texture(
    vk_app: &VkCtx,
    size: &vk::Extent2D,
    is_ping_pass: bool,
  ) -> VkTexture {
    let name = either!(is_ping_pass, "ssao_blur_tmp", "ssao");
    vk_app.create_attachment::<Self>(name, Self::RESULT_TEXTURE_FORMAT, *size)
  }

  pub fn create_framebuffer(&self, vk_app: &VkCtx, size: &vk::Extent2D) -> SSAOPassFramebuffer {
    let device = vk_app.vk_device();

    let ssao_tex = Self::create_result_texture(vk_app, &size, false);

    let fbo = create_framebuffer(device, self.render_pass, &[ssao_tex.image_view()], &size);

    SSAOPassFramebuffer { ssao_tex, fbo }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut SSAOPassFramebuffer,
    depth_stencil_tex: &mut VkTexture,
    depth_tex_image_view: vk::ImageView,
    normals_tex: &mut VkTexture,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    let config = exec_ctx.config.borrow();
    let size = config.get_ssao_viewport_size();
    let pass_name = &get_simple_type_name::<Self>();

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        framebuffer,
        depth_stencil_tex,
        normals_tex,
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

      // bind uniforms (do not move this)
      self.bind_uniforms(
        exec_ctx,
        depth_stencil_tex,
        depth_tex_image_view,
        normals_tex,
      );

      // draw calls
      cmd_draw_fullscreen_triangle(device, &command_buffer);

      // end
      exec_ctx.cmd_end_render_pass(scope_id);
    }
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    framebuffer: &mut SSAOPassFramebuffer,
    depth_stencil_tex: &mut VkTexture,
    normals_tex: &mut VkTexture,
  ) {
    VkTexture::cmd_transition_attachments_for_read_barrier(
      device,
      *command_buffer,
      &mut [depth_stencil_tex, normals_tex],
    );

    VkTexture::cmd_transition_attachments_for_write_barrier(
      device,
      *command_buffer,
      &mut [&mut framebuffer.ssao_tex],
    );
  }

  unsafe fn bind_uniforms(
    &self,
    exec_ctx: &PassExecContext,
    depth_stencil_tex: &mut VkTexture,
    depth_tex_image_view: vk::ImageView,
    normals_tex: &mut VkTexture,
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
      BindableResource::Texture {
        binding: BINDING_INDEX_NORMAL,
        texture: normals_tex,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Texture {
        binding: BINDING_INDEX_NOISE,
        texture: &self.rng_vectors_texture,
        image_view: None,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: BINDING_INDEX_KERNEL,
        buffer: &self.rng_sample_directions_kernel,
      },
    ];
    bind_resources_to_descriptors_graphic(&resouce_binder, 0, &uniform_resouces);
  }
}

pub struct SSAOPassFramebuffer {
  pub ssao_tex: VkTexture,
  pub fbo: vk::Framebuffer,
}

impl SSAOPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    self.ssao_tex.delete(device, allocator);
  }
}

fn create_random_sampling_texture(vk_app: &VkCtx, size_px: u32) -> VkTexture {
  let size = vk::Extent2D {
    width: size_px as _,
    height: size_px as _,
  };

  let data_bytes = create_random_sampling_texture_data(size);
  vk_app.create_texture_from_data(
    "SSAOPass.noiseTexture".to_string(),
    size,
    NOISE_TEXTURE_FORMAT,
    &data_bytes,
  )
}

fn create_random_sampling_texture_data(size: vk::Extent2D) -> Vec<u8> {
  let mut rng = RngVectorGenerator::new();

  VkTexture::create_texture_bytes(size, |_, _, _| {
    let tmp = rng.generate_rng_hemisphere_vector();
    vec![tmp[0], tmp[1], tmp[2], 0.0]
  })
}

fn create_random_directions_kernel(vk_app: &VkCtx, count: u32) -> VkBuffer {
  let mut rng = RngVectorGenerator::new();
  let mut data: Vec<f32> = Vec::with_capacity(count as _);

  (0..count).for_each(|idx| {
    let weight = (idx as f32) / (count as f32);
    let tmp = rng.generate_rng_hemisphere_point(weight);
    data.push(tmp[0]);
    data.push(tmp[1]);
    data.push(tmp[2]);
  });

  let data_bytes: &[u8] = bytemuck::cast_slice(&data[..]);
  vk_app.create_buffer_from_data(
    "SSAOPass.rngKernel".to_string(),
    data_bytes,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
  )
}
