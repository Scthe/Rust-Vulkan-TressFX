use ash;
use ash::vk;
use glam::Vec3;
use log::info;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::config::{Config, SSAOConfig};
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_SCENE_DEPTH: u32 = 1;
const BINDING_INDEX_NORMAL: u32 = 2;
const BINDING_INDEX_NOISE: u32 = 3;
const BINDING_INDEX_KERNEL: u32 = 4;

const RESULT_TEXTURE_FORMAT: vk::Format = vk::Format::R32_SFLOAT;
const NOISE_TEXTURE_FORMAT: vk::Format = vk::Format::R32G32B32_SFLOAT;
const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/fullscreenQuad.vert.spv",
  "./assets/shaders-compiled/ssao.frag.spv",
);

/*
TODOs
1. split config into subfolder, use default trait
  1.1 investigate texture write barriers, the stages are reversed?
  1.2 move render_graph to main dir
  1.3? scene too?
2. draw as debug display mode
3. make it work
4. use in forward pass
*/

pub struct SSAOPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
  /// Random vectors used to create tangent-bitangent-normal coordinate system
  rng_vectors_texture: VkTexture,
  /// Sampling vectors from currently shaded point
  rng_sample_directions_kernel: VkBuffer,
}

impl SSAOPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating SSAOPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = SSAOPass::create_render_pass(device);
    let uniforms_desc = SSAOPass::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline =
      SSAOPass::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    let rng_vectors_texture =
      create_random_sampling_texture(vk_app, SSAOConfig::RNG_VECTOR_TEXTURE_SIZE);
    let rng_sample_directions_kernel =
      create_random_directions_kernel(vk_app, SSAOConfig::MAX_KERNEL_VALUES);

    SSAOPass {
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
      RESULT_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::DONT_CARE, // we override every pixel regardless
      vk::AttachmentStoreOp::STORE,
      false,
    );

    let subpass = vk::SubpassDescription::builder()
      .color_attachments(&[color_attachment.1])
      .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
      .build();

    let dependencies = vk::SubpassDependency::builder()
      .src_subpass(vk::SUBPASS_EXTERNAL)
      .dst_subpass(0)
      .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
      .src_access_mask(vk::AccessFlags::empty())
      .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
      .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
      .build();

    let create_info = vk::RenderPassCreateInfo::builder()
      .dependencies(&[dependencies])
      .attachments(&[color_attachment.0])
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

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    frame_id: usize,
    config: &Config,
  ) -> SSAOPassFramebuffer {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;
    let size = config.get_ssao_viewport_size();

    let ssao_tex = VkTexture::empty(
      device,
      allocator,
      format!("SSAOPass.ssao#{}", frame_id),
      size,
      RESULT_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
      vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

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
    let size = exec_ctx.config.get_ssao_viewport_size();

    unsafe {
      self.cmd_resource_barriers(
        device,
        &command_buffer,
        framebuffer,
        depth_stencil_tex,
        normals_tex,
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
      self.bind_uniforms(
        exec_ctx,
        depth_stencil_tex,
        depth_tex_image_view,
        normals_tex,
      );

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
    framebuffer: &mut SSAOPassFramebuffer,
    depth_stencil_tex: &mut VkTexture,
    normals_tex: &mut VkTexture,
  ) {
    let depth_barrier = depth_stencil_tex.barrier_prepare_attachment_for_shader_read();
    let normals_barrier = normals_tex.barrier_prepare_attachment_for_shader_read();
    let result_barrier = framebuffer.ssao_tex.barrier_prepare_attachment_for_write();

    device.cmd_pipeline_barrier(
      *command_buffer,
      // wait for this
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS
        | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
      // before we execute fragment shader
      vk::PipelineStageFlags::FRAGMENT_SHADER,
      vk::DependencyFlags::empty(),
      &[],
      &[],
      &[depth_barrier, normals_barrier, result_barrier],
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
      BindableResource::Uniform {
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
      BindableResource::Uniform {
        binding: BINDING_INDEX_KERNEL,
        buffer: &self.rng_sample_directions_kernel,
      },
    ];
    bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);
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

  VkTexture::from_data(
    vk_app.vk_device(),
    &vk_app.allocator,
    vk_app,
    "SSAOPass.noiseTexture".to_string(),
    NOISE_TEXTURE_FORMAT,
    size,
    &data_bytes,
  )
}

fn create_random_sampling_texture_data(size: vk::Extent2D) -> Vec<u8> {
  let mut rng = RngVectorGenerator::new();

  VkTexture::create_texture_bytes(size, |_, _, idx| {
    let mut tmp = rng.generate_rng_hemisphere_vector();

    // get_random_point_in_hemisphere
    // ATM points lie on edge of sphere, randomize then inside
    let kernel_size: f32 = (size.width * size.height) as _;
    let mut scale_fac = (idx as f32) / kernel_size;
    scale_fac = 0.1 + 0.9 * scale_fac * scale_fac; // lerp(0.1, 1.0, scale_fac * scale_fac);
    tmp = tmp * scale_fac;

    vec![tmp[0], tmp[1], tmp[2]]
  })
}

fn create_random_directions_kernel(vk_app: &VkCtx, count: u32) -> VkBuffer {
  let mut rng = RngVectorGenerator::new();
  let mut data: Vec<f32> = Vec::with_capacity(count as _);

  (0..count).for_each(|idx| {
    let mut tmp = rng.generate_rng_hemisphere_vector();

    // get_random_point_in_hemisphere
    // ATM points lie on edge of sphere, randomize then inside
    let mut scale_fac = (idx as f32) / (count as f32);
    scale_fac = 0.1 + 0.9 * scale_fac * scale_fac; // lerp(0.1, 1.0, scale_fac * scale_fac);
    tmp = tmp * scale_fac;
    data.push(tmp[0]);
    data.push(tmp[1]);
    data.push(tmp[2]);
  });

  let data_bytes: &[u8] = bytemuck::cast_slice(&data[..]);
  VkBuffer::from_data(
    "SSAOPass.rngKernel".to_string(),
    data_bytes,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
    &vk_app.allocator,
    vk_app.device.queue_family_index,
  )
}

struct RngVectorGenerator {
  rng: ThreadRng,
}

impl RngVectorGenerator {
  pub fn new() -> Self {
    Self {
      rng: rand::thread_rng(),
    }
  }

  /// Get random direction in hemisphere.
  /// Not exactly uniform distribution, but meh..
  pub fn generate_rng_hemisphere_vector(&mut self) -> Vec3 {
    let tmp = glam::vec3(
      self.rng.gen::<f32>() * 2.0 - 1.0, // [-1, 1]
      self.rng.gen::<f32>() * 2.0 - 1.0, // [-1, 1]
      self.rng.gen::<f32>(),             // [0, 1], HEMIsphere, not full sphere
    );
    tmp.normalize()
  }
}
