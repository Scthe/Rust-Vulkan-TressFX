use ash;
use ash::vk;
use log::trace;

use crate::render_graph::_shared::RenderableVertex;
use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

const BINDING_INDEX_CONFIG_UBO: u32 = 0;
const BINDING_INDEX_DIFFUSE_TEXTURE: u32 = 1;

const DEPTH_TEXTURE_FORMAT: vk::Format = vk::Format::D24_UNORM_S8_UINT;
const DIFFUSE_TEXTURE_FORMAT: vk::Format = vk::Format::R32G32B32A32_SFLOAT;

pub struct ForwardPass {
  pub render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl ForwardPass {
  pub fn new(vk_app: &VkCtx) -> Self {
    trace!("Creating ForwardPass");
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let render_pass = ForwardPass::create_render_pass(device);
    let uniforms_layout = ForwardPass::create_uniforms_layout(device);
    let (pipeline, pipeline_layout) =
      ForwardPass::create_pipeline(device, pipeline_cache, &render_pass, &[uniforms_layout]);

    ForwardPass {
      render_pass,
      pipeline,
      pipeline_layout,
      uniforms_layout,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_render_pass(self.render_pass, None);
    device.destroy_descriptor_set_layout(self.uniforms_layout, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
  }

  fn create_render_pass(device: &ash::Device) -> vk::RenderPass {
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

    let subpass = vk::SubpassDescription::builder()
      .color_attachments(&[color_attachment.1])
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
      .attachments(&[depth_attachment.0, color_attachment.0])
      .subpasses(&[subpass])
      .build();
    let render_pass = unsafe {
      device
        .create_render_pass(&create_info, None)
        .expect("Failed creating render pass")
    };

    render_pass
  }

  fn create_uniforms_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    let binding_config_ubo =
      create_ubo_binding(BINDING_INDEX_CONFIG_UBO, vk::ShaderStageFlags::VERTEX);
    let binding_diff_tex = create_texture_binding(
      BINDING_INDEX_DIFFUSE_TEXTURE,
      vk::ShaderStageFlags::FRAGMENT,
    );

    let ubo_descriptors_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
      .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR)
      .bindings(&[binding_config_ubo, binding_diff_tex])
      .build();

    unsafe {
      device
        .create_descriptor_set_layout(&ubo_descriptors_create_info, None)
        .expect("Failed to create DescriptorSetLayout")
    }
  }

  fn create_pipeline(
    device: &ash::Device,
    pipeline_cache: &vk::PipelineCache,
    render_pass: &vk::RenderPass,
    uniform_layouts: &[vk::DescriptorSetLayout],
  ) -> (vk::Pipeline, vk::PipelineLayout) {
    // pipeline layout
    let pipeline_layout = create_pipeline_layout(device, uniform_layouts);

    // create shaders
    let (module_vs, stage_vs, module_fs, stage_fs) = load_render_shaders(
      device,
      "./assets/shaders-compiled/forward.vert.spv",
      "./assets/shaders-compiled/forward.frag.spv",
    );

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
      .vertex_attribute_descriptions(&RenderableVertex::get_attributes_descriptions())
      .vertex_binding_descriptions(&RenderableVertex::get_bindings_descriptions())
      .build();

    let dynamic_state = ps_dynamic_state(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    // create pipeline itself
    // TODO cull backfaces
    let color_attachment_count: usize = 1;
    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
      .stages(&[stage_vs, stage_fs])
      .vertex_input_state(&vertex_input_state)
      .input_assembly_state(&ps_ia_triangle_list())
      .viewport_state(&ps_viewport_single_dynamic())
      .rasterization_state(&ps_raster_polygons(vk::CullModeFlags::NONE))
      .multisample_state(&ps_multisample_disabled())
      .depth_stencil_state(&ps_depth_less_stencil_always())
      .color_blend_state(&ps_color_blend_override(color_attachment_count))
      .dynamic_state(&dynamic_state)
      .layout(pipeline_layout)
      .render_pass(*render_pass)
      .build();

    let pipeline = create_pipeline(device, pipeline_cache, pipeline_create_info);

    unsafe {
      device.destroy_shader_module(module_vs, None);
      device.destroy_shader_module(module_fs, None);
    }

    (pipeline, pipeline_layout)
  }

  pub fn create_framebuffer(&self, vk_app: &VkCtx, size: &vk::Extent2D) -> ForwardPassFramebuffer {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    let depth_tex = VkTexture::new(
      device,
      allocator,
      format!("ForwardPass.depth#???"),
      *size,
      DEPTH_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
      vk::ImageAspectFlags::DEPTH,
    );
    let diffuse_tex = VkTexture::new(
      device,
      allocator,
      format!("ForwardPass.difuse#???"),
      *size,
      DIFFUSE_TEXTURE_FORMAT,
      vk::ImageTiling::OPTIMAL,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
      vk::ImageAspectFlags::COLOR,
    );

    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[depth_tex.image_view(), diffuse_tex.image_view()],
      &size,
    );

    ForwardPassFramebuffer {
      depth_tex,
      diffuse_tex,
      fbo,
    }
  }

  pub fn execute(
    &self,
    vk_app: &VkCtx,
    scene: &World,
    command_buffer: vk::CommandBuffer,
    framebuffer: &mut ForwardPassFramebuffer,
    size: vk::Extent2D,
    config_buffer: &VkBuffer,
  ) -> () {
    let device = vk_app.vk_device();
    let push_descriptor = &vk_app.push_descriptor;
    let render_area = size_to_rect_vk(&size);
    let viewport = create_viewport(&size);
    let clear_color = vk::ClearColorValue {
      float32: [0.2f32, 0.2f32, 0.2f32, 1f32],
    };
    let clear_depth = vk::ClearDepthStencilValue {
      depth: 1.0f32,
      stencil: 0,
    };

    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
      .render_pass(self.render_pass)
      .framebuffer(framebuffer.fbo)
      .render_area(render_area)
      .clear_values(&[
        vk::ClearValue {
          depth_stencil: clear_depth,
        },
        vk::ClearValue { color: clear_color },
      ])
      .build();

    // TODO no need to rerecord every frame TBH. Everything can be controlled by uniforms etc.
    unsafe {
      let texture_barrier = framebuffer.diffuse_tex.prepare_for_layout_transition(
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        vk::AccessFlags::SHADER_READ,
      );
      device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[texture_barrier],
      );

      // start render pass
      device.cmd_begin_render_pass(
        command_buffer,
        &render_pass_begin_info,
        vk::SubpassContents::INLINE,
      );

      // draw calls go here
      device.cmd_set_viewport(command_buffer, 0, &[viewport]);
      device.cmd_set_scissor(command_buffer, 0, &[render_area]);
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        self.pipeline,
      );

      // bind uniforms
      let resouce_binder = ResouceBinder {
        push_descriptor,
        command_buffer,
        pipeline_layout: self.pipeline_layout,
      };

      // draw calls
      for entity in &scene.entities {
        // uniforms
        let uniform_resouces = [
          BindableResource::Uniform {
            binding: BINDING_INDEX_CONFIG_UBO,
            buffer: config_buffer,
          },
          BindableResource::Texture {
            binding: BINDING_INDEX_DIFFUSE_TEXTURE,
            texture: &entity.tex_diffuse,
            sampler: vk_app.default_texture_sampler_linear,
          },
        ];
        bind_resources_to_descriptors(&resouce_binder, 0, &uniform_resouces);

        // draw mesh
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
}

pub struct ForwardPassFramebuffer {
  pub depth_tex: VkTexture,
  pub diffuse_tex: VkTexture,
  // pub normals_tex: VkTexture,
  pub fbo: vk::Framebuffer,
}

impl ForwardPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    self.depth_tex.delete(device, allocator);
    self.diffuse_tex.delete(device, allocator);
  }
}
