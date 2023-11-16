use std::mem::size_of;

use ash;
use ash::vk;
use glam::{vec4, Mat4, Vec3, Vec4};
use log::info;

use crate::config::ShadowSourceCfg;
use crate::render_graph::tfx_render::TfxForwardPass;
use crate::scene::{Camera, TfxObject};
use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::{PassExecContext, RenderableVertex};

const DEPTH_TEXTURE_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
const COLOR_ATTACHMENT_COUNT: usize = 1;
const SHADER_PATHS_MESHES: (&str, &str) = (
  "./assets/shaders-compiled/shadow_map_gen.vert.spv",
  "./assets/shaders-compiled/shadow_map_gen.frag.spv",
);
const SHADER_PATHS_TRESSFX_HAIR: (&str, &str) = (
  "./assets/shaders-compiled/tfx_shadow_map_gen.vert.spv",
  "./assets/shaders-compiled/shadow_map_gen.frag.spv",
);

/// Render depth map from the point of view of `ShadowSource`. Quite flexible honestly,
/// many other effects might want to reuse it (e.g. SSS depth pass).
pub struct ShadowMapPass {
  render_pass: vk::RenderPass,
  pipeline_meshes: vk::Pipeline,
  pipeline_layout_meshes: vk::PipelineLayout,
  pipeline_hair: vk::Pipeline,
  pipeline_layout_hair: vk::PipelineLayout,
  uniforms_layout_hair: vk::DescriptorSetLayout,
}

impl ShadowMapPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating {}", get_simple_type_name::<Self>());
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = Self::create_render_pass(device);
    let push_constant_ranges = Self::get_push_constant_layout();

    // meshes
    let pipeline_layout_meshes = create_pipeline_layout(device, &[], &[push_constant_ranges]);
    let pipeline_meshes = Self::create_pipeline_meshes(
      device,
      pipeline_cache,
      &render_pass,
      &pipeline_layout_meshes,
    );

    // hair
    let uniforms_desc_hair = Self::get_uniforms_layout_hair();
    let uniforms_layout_hair = create_push_descriptor_layout(device, uniforms_desc_hair);
    let pipeline_layout_hair =
      create_pipeline_layout(device, &[uniforms_layout_hair], &[push_constant_ranges]);
    let pipeline_hair = Self::create_pipeline_tressfx_hair(
      device,
      pipeline_cache,
      &render_pass,
      &pipeline_layout_hair,
    );

    // finish (finally!)
    Self {
      render_pass,
      pipeline_meshes,
      pipeline_layout_meshes,
      pipeline_hair,
      pipeline_layout_hair,
      uniforms_layout_hair,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_pipeline_layout(self.pipeline_layout_meshes, None);
    device.destroy_pipeline(self.pipeline_meshes, None);
    device.destroy_pipeline_layout(self.pipeline_layout_hair, None);
    device.destroy_pipeline(self.pipeline_hair, None);
    device.destroy_descriptor_set_layout(self.uniforms_layout_hair, None);
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

  fn get_push_constant_layout() -> vk::PushConstantRange {
    vk::PushConstantRange::builder()
      .offset(0)
      .size(size_of::<ShadowMapPassPushConstants>() as _)
      .stage_flags(vk::ShaderStageFlags::VERTEX)
      .build()
  }

  fn create_pipeline_meshes(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    pipeline_layout: &vk::PipelineLayout,
  ) -> vk::Pipeline {
    let vertex_desc = RenderableVertex::get_vertex_description();
    Self::create_pipeline(
      device,
      pipeline_cache,
      render_pass,
      pipeline_layout,
      SHADER_PATHS_MESHES,
      vertex_desc,
    )
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
      SHADER_PATHS_TRESSFX_HAIR,
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
        let mut attachment_blends =
          Vec::<vk::PipelineColorBlendAttachmentState>::with_capacity(COLOR_ATTACHMENT_COUNT);

        let pipeline_create_info = builder
          .depth_stencil_state(&ps_depth_less_stencil_always())
          // see https://docs.microsoft.com/en-us/windows/desktop/DxTechArts/common-techniques-to-improve-shadow-depth-maps#back-face-and-front-face
          .rasterization_state(&ps_raster_polygons(vk::CullModeFlags::NONE))
          // disable writes to color (we do not set any attachments anyway)
          .color_blend_state(&ps_color_blend_override(&mut attachment_blends, COLOR_ATTACHMENT_COUNT, vk::ColorComponentFlags::empty()))
          .build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  /// - @param `PassType` - use when using ShadowPass as impl. detail of other passes. Unfortunately, we cannot default to `Self`
  pub fn create_framebuffer<PassType>(
    &self,
    vk_app: &VkCtx,
    frame_id: usize,
    size_px: u32,
  ) -> ShadowMapPassFramebuffer {
    let device = vk_app.vk_device();
    let size = vk::Extent2D {
      width: size_px,
      height: size_px,
    };

    let depth_tex =
      vk_app.create_attachment::<PassType>("depth", frame_id, DEPTH_TEXTURE_FORMAT, size);

    let fbo = create_framebuffer(device, self.render_pass, &[depth_tex.image_view()], &size);

    ShadowMapPassFramebuffer { depth_tex, fbo }
  }

  /// - @param `PassType` - use when using ShadowPass as impl. detail of other passes. Unfortunately, we cannot default to `Self`
  pub fn execute<PassType>(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut ShadowMapPassFramebuffer,
    shadow_source: &ShadowSourceCfg,
    render_hair: bool,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    let pass_name = &get_simple_type_name::<PassType>();

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
      let scope_id = exec_ctx.cmd_start_render_pass(
        pass_name,
        &self.render_pass,
        &framebuffer.fbo,
        &size,
        &[clear_depth],
      );

      // draw meshes
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline_meshes,
      );
      let scene = &*exec_ctx.scene;
      for entity in &scene.entities {
        self.bind_push_constants(
          exec_ctx,
          shadow_source,
          &entity.model_matrix,
          1.0,
          shadow_source.position(),
          size,
        );
        entity.cmd_bind_mesh_buffers(device, command_buffer);
        entity.cmd_draw_mesh(device, command_buffer);
      }

      // draw hair
      if render_hair {
        device.cmd_bind_pipeline(
          command_buffer,
          vk::PipelineBindPoint::GRAPHICS,
          self.pipeline_hair,
        );
        for entity in &scene.tressfx_objects {
          self.bind_hair_ubos(exec_ctx, entity);
          self.bind_push_constants(
            exec_ctx,
            shadow_source,
            &entity.model_matrix,
            exec_ctx.config.shadows.hair_tfx_radius_multipler,
            shadow_source.position(),
            size,
          );
          entity.cmd_draw_mesh(device, command_buffer);
        }
      }

      // end
      exec_ctx.cmd_end_render_pass(scope_id);
    }
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    framebuffer: &mut ShadowMapPassFramebuffer,
  ) {
    VkTexture::cmd_transition_attachments_for_write_barrier(
      device,
      *command_buffer,
      &mut [&mut framebuffer.depth_tex],
    );
  }

  unsafe fn bind_push_constants(
    &self,
    exec_ctx: &PassExecContext,
    source: &ShadowSourceCfg,
    model_matrix: &Mat4,
    hair_fiber_radius: f32,
    camera_position: Vec3,
    shadowmap_size: vk::Extent2D,
  ) {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();

    // push constants
    let push_constants = ShadowMapPassPushConstants {
      mvp: Self::get_light_shadow_mvp(source, *model_matrix),
      camera_position: vec4(
        camera_position.x,
        camera_position.y,
        camera_position.z,
        hair_fiber_radius,
      ),
      viewport: vec4(
        shadowmap_size.width as f32,
        shadowmap_size.height as f32,
        0.0,
        0.0,
      ),
    };
    let push_constants_bytes = bytemuck::bytes_of(&push_constants);
    device.cmd_push_constants(
      command_buffer,
      self.pipeline_layout_meshes,
      vk::ShaderStageFlags::VERTEX,
      0,
      push_constants_bytes,
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

    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout_hair);
    bind_resources_to_descriptors_graphic(&resouce_binder, 0, &uniform_resouces);
  }

  pub fn get_light_shadow_mvp(source: &ShadowSourceCfg, model_matrix: Mat4) -> Mat4 {
    let v_mat = get_depth_view_matrix(source);
    let p_mat = get_depth_projection_matrix(source);
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
  camera_position: Vec4,
  viewport: Vec4,
}

unsafe impl bytemuck::Zeroable for ShadowMapPassPushConstants {}
unsafe impl bytemuck::Pod for ShadowMapPassPushConstants {}

///////////////////////////////
// matrices calc

fn get_depth_view_matrix(source: &ShadowSourceCfg) -> Mat4 {
  return Mat4::look_at_rh(source.position(), source.look_at_target, Vec3::Y);
}

fn get_depth_projection_matrix(source: &ShadowSourceCfg) -> Mat4 {
  // this is for directional light, all rays are parallel
  let dpm = &source.projection;
  Mat4::orthographic_rh(dpm.left, dpm.right, dpm.bottom, dpm.top, dpm.near, dpm.far)
}
