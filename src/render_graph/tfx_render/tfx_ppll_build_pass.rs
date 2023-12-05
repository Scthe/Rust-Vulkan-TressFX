use ash;
use ash::vk;
use log::info;

use crate::config::Config;
use crate::render_graph::forward_pass::ForwardPass;
use crate::scene::TfxObject;
use crate::utils::create_per_object_pass_name;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use crate::render_graph::PassExecContext;

const SHADER_PATHS: (&str, &str) = (
  "./assets/shaders-compiled/tfx_ppll_build.vert.spv",
  "./assets/shaders-compiled/tfx_ppll_build.frag.spv",
);

/// Build per pixel linked list. For each pixel it adds a list of data of all
/// hair strands that intersect this pixel. We will then resolve this list
/// (in a next pass) and combine all gathered hair strands
/// to get a single final color.
///
/// It's a solution for order-independent transparency.
/// https://github.com/Scthe/TressFX-OpenGL/blob/master/src/gl-tfx/TFxPPLL.cpp
/// https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L554
pub struct TfxPpllBuildPass {
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl TfxPpllBuildPass {
  /// Must match shader definition (4 uints + 4 f32 == 32 bytes)
  const PPLL_NODE_BYTES: u32 = 32;
  const PPLL_AVG_NODES_PER_PIXEL: u32 = 4;
  const PPLL_ATOMIC_COUNTER_BYTES: usize = 4; // single uint
  /// Must match shader value
  const PPLL_FRAGMENT_LIST_NULL: u32 = 0xffffffff;
  const PPLL_COUNTER_RESET_VALUE: u32 = 0;

  const BINDING_INDEX_CONFIG_UBO: u32 = 0;
  const BINDING_INDEX_POSITIONS_SSBO: u32 = 1;
  const BINDING_INDEX_TANGENTS_SSBO: u32 = 2;
  const BINDING_INDEX_TFX_PARAMS_UBO: u32 = 3;
  const BINDING_INDEX_HEAD_POINTERS_IMAGE: u32 = 4;
  const BINDING_INDEX_DATA_BUFFER: u32 = 5;
  const BINDING_INDEX_NEXT_FREE_ENTRY_ATOMIC: u32 = 6;

  const COLOR_ATTACHMENT_COUNT: usize = 0;

  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating TfxPpllBuildPass");
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

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
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

    return unsafe { create_render_pass_from_attachments(device, Some(depth_attachment), &[]) };
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
      create_storage_image_binding(
        Self::BINDING_INDEX_HEAD_POINTERS_IMAGE,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_DATA_BUFFER,
        vk::ShaderStageFlags::FRAGMENT,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_NEXT_FREE_ENTRY_ATOMIC,
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
    let vertex_desc = ps_vertex_empty();

    create_pipeline_with_defaults(
      device,
      render_pass,
      pipeline_layout,
      SHADER_PATHS,
      vertex_desc,
      Self::COLOR_ATTACHMENT_COUNT,
      |builder| {
        let stencil_write_hair = ps_stencil_write_if_depth_passed(Config::STENCIL_BIT_HAIR, true);
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
          .depth_test_enable(true)
          .depth_write_enable(false)
          .depth_compare_op(vk::CompareOp::LESS)
          .depth_bounds_test_enable(false)
          .stencil_test_enable(true)
          .front(stencil_write_hair)
          .back(stencil_write_hair)
          .build();

        let mut attachment_blends =
          Vec::<vk::PipelineColorBlendAttachmentState>::with_capacity(Self::COLOR_ATTACHMENT_COUNT);

        let pipeline_create_info = builder
          .depth_stencil_state(&depth_stencil)
          .color_blend_state(&ps_color_blend_override(
            &mut attachment_blends,
            Self::COLOR_ATTACHMENT_COUNT,
            vk::ColorComponentFlags::empty(),
          ))
          .build();
        create_pipeline(device, pipeline_cache, pipeline_create_info)
      },
    )
  }

  /// How many elements can be allocated in `u_linkedListDataBuffer`.
  /// Size of the allocated PPLL fragment data buffer (in elements).
  /// @return width * height * AVG_FRAGS_PER_PIXEL(4)
  pub fn get_ppll_data_nodes_count(size: vk::Extent2D) -> u32 {
    size.width * size.height * Self::PPLL_AVG_NODES_PER_PIXEL
  }

  pub fn create_framebuffer(
    &self,
    vk_app: &VkCtx,
    depth_stencil_tex: &VkTexture,
  ) -> TfxPpllBuildPassFramebuffer {
    let device = vk_app.vk_device();
    let size = depth_stencil_tex.size();
    let fbo = create_framebuffer(
      device,
      self.render_pass,
      &[depth_stencil_tex.image_view()],
      &size,
    );

    // head pointers texture
    // https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L228
    let head_pointers_image = vk_app.create_texture_empty(
      create_per_object_pass_name::<Self>("head_pointers_image"),
      size,
      vk::Format::R32_UINT,
      vk::ImageTiling::OPTIMAL,
      // will be cleared every frame:
      vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_DST,
      VkMemoryPreference::GpuOnly,
      vk::ImageLayout::GENERAL,
    );

    // ppll data
    // https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L281
    let ppll_size = Self::get_ppll_data_nodes_count(size) * Self::PPLL_NODE_BYTES;
    let ppll_data = vk_app.create_buffer_empty(
      create_per_object_pass_name::<Self>("ppll_data"),
      ppll_size as _,
      vk::BufferUsageFlags::STORAGE_BUFFER,
      VkMemoryPreference::GpuOnly,
    );

    // single atomic uint
    let ppll_next_free_entry_atomic = vk_app.create_buffer_empty(
      create_per_object_pass_name::<Self>("ppll_next_free_entry_atomic"),
      Self::PPLL_ATOMIC_COUNTER_BYTES,
      // will be cleared every frame:
      vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
      VkMemoryPreference::GpuOnly,
    );

    TfxPpllBuildPassFramebuffer {
      fbo,
      head_pointers_image,
      ppll_data,
      ppll_next_free_entry_atomic,
    }
  }

  pub fn execute(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut TfxPpllBuildPassFramebuffer,
    depth_stencil_tex: &mut VkTexture,
    entity: &TfxObject,
  ) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let size = exec_ctx.size;
    let device = vk_app.vk_device();
    let pass_name = &create_per_object_pass_name::<Self>(&entity.name);

    unsafe {
      // profiling might be a bit skewed cause barriers
      let scope_id = exec_ctx.cmd_begin_scope(pass_name);

      // clears not allowed inside render pass per Vulkan Spec 19.1
      // https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html#clears
      self.cmd_reset_current_values(exec_ctx, framebuffer);

      // usuall sync stuff
      self.cmd_resource_barriers(device, &command_buffer, depth_stencil_tex);

      // start render pass
      exec_ctx.cmd_start_render_pass(
        &self.render_pass,
        &self.pipeline,
        &framebuffer.fbo,
        &size,
        &[],
      );

      // draw calls
      self.bind_entity_ubos(exec_ctx, framebuffer, entity);
      entity.cmd_draw_mesh(device, command_buffer);

      // end
      exec_ctx.cmd_end_render_pass(scope_id);
    }
  }

  unsafe fn cmd_reset_current_values(
    &self,
    exec_ctx: &PassExecContext,
    framebuffer: &mut TfxPpllBuildPassFramebuffer,
  ) {
    let vk_app = exec_ctx.vk_app;
    let device = vk_app.vk_device();
    let command_buffer = exec_ctx.command_buffer;

    // We could try to do a sync here, but it's pointless. It's across many frames,
    // which are inheritely behind semaphores.

    // clear atomic counter to 0
    // After user presses 'reset simulation' buttonm the "QueueSubmit sync. validation layer"
    // claims SYNC-HAZARD-WRITE-AFTER-READ. But simulation reset does not touch
    // `ppll_next_free_entry_atomic`. Since "QueueSubmit sync ..." is marked as (ALPHA)
    // we can ignore.
    device.cmd_fill_buffer(
      command_buffer,
      framebuffer.ppll_next_free_entry_atomic.buffer,
      0,
      vk::WHOLE_SIZE,
      Self::PPLL_COUNTER_RESET_VALUE,
    );

    // reset heads texture
    // https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L554
    // Triggers [UNASSIGNED-BestPractices-ClearColor-NotCompressed] in validation layers,
    // but that's ok - we have our own special value to clear with.
    let clear_color_value = vk::ClearColorValue {
      uint32: [
        Self::PPLL_FRAGMENT_LIST_NULL,
        Self::PPLL_FRAGMENT_LIST_NULL,
        Self::PPLL_FRAGMENT_LIST_NULL,
        Self::PPLL_FRAGMENT_LIST_NULL,
      ],
    };
    let range = vk::ImageSubresourceRange {
      aspect_mask: framebuffer.head_pointers_image.aspect_flags,
      base_mip_level: 0,
      level_count: 1, // mip_level_count
      base_array_layer: 0,
      layer_count: 1,
    };
    device.cmd_clear_color_image(
      command_buffer,
      framebuffer.head_pointers_image.image,
      framebuffer.head_pointers_image.layout,
      &clear_color_value,
      &[range],
    );
  }

  unsafe fn cmd_resource_barriers(
    &self,
    device: &ash::Device,
    command_buffer: &vk::CommandBuffer,
    depth_stencil_tex: &mut VkTexture, // write
  ) {
    // Both STORAGE_IMAGE and SSBO!
    // After cmd_fill_buffer and cmd_clear_color_image
    // https://github.com/SaschaWillems/Vulkan/blob/master/examples/oit/oit.cpp#L559
    let barrier = VkStorageResourceBarrier {
      previous_op: (
        vk::PipelineStageFlags2::CLEAR // vkCmdClearColorImage
          | vk::PipelineStageFlags2::TRANSFER, // vkCmdFillBuffer
        vk::AccessFlags2::TRANSFER_WRITE,
      ),
      next_op: (
        vk::PipelineStageFlags2::FRAGMENT_SHADER,
        vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
      ),
    };
    cmd_storage_resource_barrier(device, *command_buffer, barrier);

    VkTexture::cmd_transition_attachments_for_write_barrier(
      device,
      *command_buffer,
      &mut [depth_stencil_tex],
    );
  }

  unsafe fn bind_entity_ubos(
    &self,
    exec_ctx: &PassExecContext,
    fbo: &mut TfxPpllBuildPassFramebuffer,
    entity: &TfxObject,
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
      BindableResource::StorageImage {
        binding: Self::BINDING_INDEX_HEAD_POINTERS_IMAGE,
        texture: &fbo.head_pointers_image,
        sampler: vk_app.default_texture_sampler_nearest,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_DATA_BUFFER,
        buffer: &fbo.ppll_data,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_NEXT_FREE_ENTRY_ATOMIC,
        buffer: &fbo.ppll_next_free_entry_atomic,
      },
    ];

    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
    bind_resources_to_descriptors_graphic(&resouce_binder, 0, &uniform_resouces);
  }
}

pub struct TfxPpllBuildPassFramebuffer {
  pub fbo: vk::Framebuffer,
  pub head_pointers_image: VkTexture,
  pub ppll_data: VkBuffer,
  pub ppll_next_free_entry_atomic: VkBuffer,
}

impl TfxPpllBuildPassFramebuffer {
  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();
    let allocator = &vk_app.allocator;

    device.destroy_framebuffer(self.fbo, None);
    self.head_pointers_image.delete(device, allocator);
    self.ppll_data.delete(allocator);
    self.ppll_next_free_entry_atomic.delete(allocator);
  }
}
