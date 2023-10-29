use std::mem::size_of;

use ash;
use ash::vk;
use glam::{Mat4, Vec3};
use log::info;

use crate::config::Config;
use crate::scene::{Camera, WorldEntity};
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::{PassExecContext, RenderableVertex};

const DEPTH_TEXTURE_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/shadow_map_gen.vert.spv",
  "./assets/shaders-compiled/shadow_map_gen.frag.spv",
);

// TODO verify
// TODO add ui
pub struct ShadowMapPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
}

impl ShadowMapPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating ShadowMapPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = ShadowMapPass::create_render_pass(device);
    let push_constant_ranges = ShadowMapPass::get_push_constant_layout();
    let pipeline_layout = create_pipeline_layout(device, &[], &[push_constant_ranges]);
    let pipeline =
      ShadowMapPass::create_pipeline(device, pipeline_cache, &render_pass, &pipeline_layout);

    ShadowMapPass {
      render_pass,
      pipeline,
      pipeline_layout,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
  }

  fn create_render_pass(device: &ash::Device) -> vk::RenderPass {
    let depth_attachment = create_depth_stencil_attachment(
      0,
      DEPTH_TEXTURE_FORMAT,
      vk::AttachmentLoadOp::CLEAR,      // depth_load_op
      vk::AttachmentStoreOp::STORE,     // depth_store_op
      vk::AttachmentLoadOp::DONT_CARE,  // stencil_load_op
      vk::AttachmentStoreOp::DONT_CARE, // stencil_store_op
      vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
    );

    let subpass = vk::SubpassDescription::builder()
      .color_attachments(&[])
      .depth_stencil_attachment(&depth_attachment.1)
      .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
      .build();

    let dependencies = vk::SubpassDependency::builder()
      .src_subpass(vk::SUBPASS_EXTERNAL)
      .dst_subpass(0)
      .src_stage_mask(vk::PipelineStageFlags::LATE_FRAGMENT_TESTS)
      .src_access_mask(vk::AccessFlags::empty())
      .dst_stage_mask(vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
      .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
      .build();

    let create_info = vk::RenderPassCreateInfo::builder()
      .dependencies(&[dependencies])
      .attachments(&[depth_attachment.0])
      .subpasses(&[subpass])
      .build();
    let render_pass = unsafe {
      device
        .create_render_pass(&create_info, None)
        .expect("Failed creating render pass")
    };

    render_pass
  }

  fn get_push_constant_layout() -> vk::PushConstantRange {
    vk::PushConstantRange::builder()
      .offset(0)
      .size(size_of::<ShadowMapPassPushConstants>() as _)
      .stage_flags(vk::ShaderStageFlags::VERTEX)
      .build()
  }

  fn create_pipeline(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    pipeline_layout: &vk::PipelineLayout,
  ) -> vk::Pipeline {
    // TODO duplicate from forward_pass, but references prevent fn extraction
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
        let pipeline_create_info = builder
          .depth_stencil_state(&ps_depth_less_stencil_always())
          // see https://docs.microsoft.com/en-us/windows/desktop/DxTechArts/common-techniques-to-improve-shadow-depth-maps#back-face-and-front-face
          .rasterization_state(&ps_raster_polygons(vk::CullModeFlags::NONE))
          // disable writes to color (we do not set any attachments anyway)
          .color_blend_state(&ps_color_blend_override(COLOR_ATTACHMENT_COUNT, vk::ColorComponentFlags::empty()))
          .build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    frame_id: usize,
    size_px: u32,
  ) -> ShadowMapPassFramebuffer {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;
    let size = vk::Extent2D {
      width: size_px,
      height: size_px,
    };

    let depth_tex = VkTexture::empty(
      device,
      allocator,
      vk_app,
      format!("ShadowMapPass.depth#{}", frame_id),
      size,
      DEPTH_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::DEPTH,
      vk::MemoryPropertyFlags::DEVICE_LOCAL,
      vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL,
    );

    let fbo = create_framebuffer(device, self.render_pass, &[depth_tex.image_view()], &size);

    ShadowMapPassFramebuffer { depth_tex, fbo }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut ShadowMapPassFramebuffer,
    light_position: Vec3,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let scene = exec_ctx.scene;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    let clear_depth = vk::ClearValue {
      depth_stencil: vk::ClearDepthStencilValue {
        depth: 1.0,
        stencil: 0u32,
      },
    };
    let size = vk::Extent2D {
      width: framebuffer.depth_tex.width,
      height: framebuffer.depth_tex.height,
    };

    unsafe {
      self.cmd_resource_barriers(device, &command_buffer, framebuffer);

      // start render pass
      cmd_begin_render_pass_for_framebuffer(
        &device,
        &command_buffer,
        &self.render_pass,
        &framebuffer.fbo,
        &size,
        &[clear_depth],
      );
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // draw calls
      for entity in &scene.entities {
        self.bind_entity_ubos(exec_ctx, entity, light_position);
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
    framebuffer: &mut ShadowMapPassFramebuffer,
  ) {
    let depth_barrier = framebuffer.depth_tex.barrier_prepare_attachment_for_write();
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
      &[depth_barrier],
    );
  }

  unsafe fn bind_entity_ubos(
    &self,
    exec_ctx: &PassExecContext,
    entity: &WorldEntity,
    light_position: Vec3,
  ) {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    // push constants
    let push_constants = ShadowMapPassPushConstants {
      mvp: Self::get_light_shadow_mvp(exec_ctx.config, entity.model_matrix, light_position),
    };
    let push_constants_bytes = bytemuck::bytes_of(&push_constants);
    device.cmd_push_constants(
      command_buffer,
      self.pipeline_layout,
      vk::ShaderStageFlags::VERTEX,
      0,
      push_constants_bytes,
    );
  }

  pub fn get_light_shadow_mvp(cfg: &Config, model_matrix: Mat4, light_pos: Vec3) -> Mat4 {
    let v_mat = get_depth_view_matrix(cfg, light_pos);
    let p_mat = get_depth_projection_matrix(cfg);
    Camera::calc_model_view_projection_matrix(&model_matrix, &v_mat, &p_mat)
  }
}

pub struct ShadowMapPassFramebuffer {
  /// Values are negative ([-near, -far]) due to Vulkan coordinate system
  pub depth_tex: VkTexture,
  pub fbo: vk::Framebuffer,
}

impl ShadowMapPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    self.depth_tex.delete(device, allocator);
  }
}

#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
struct ShadowMapPassPushConstants {
  mvp: Mat4,
}

unsafe impl bytemuck::Zeroable for ShadowMapPassPushConstants {}
unsafe impl bytemuck::Pod for ShadowMapPassPushConstants {}

///////////////////////////////
// matrices calc

fn get_depth_view_matrix(cfg: &Config, light_pos: Vec3) -> Mat4 {
  let dl = &cfg.shadows.shadow_source;
  return Mat4::look_at_rh(light_pos, dl.look_at_target, Vec3::Y); // target - both shadows and SSS
}

fn get_depth_projection_matrix(cfg: &Config) -> Mat4 {
  // this is for directional light, all rays are parallel
  let dpm = &cfg.shadows.shadow_source.projection; // both shadows and SSS
  Mat4::orthographic_rh(dpm.left, dpm.right, dpm.bottom, dpm.top, dpm.near, dpm.far)
}
