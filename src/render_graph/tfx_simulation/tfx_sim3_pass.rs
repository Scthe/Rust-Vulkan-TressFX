use ash;
use ash::vk;
use log::info;

use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;
use crate::{scene::TfxObject, utils::create_per_object_pass_name};

use super::{group_count_x_per_vertex, PassExecContext, TfxSim0Pass};

const SHADER_PATH: &str =
  "./assets/shaders-compiled/sim3_LengthConstraintsWindAndCollision.comp.spv";

/// ### Compute shader for: wind + collisions etc.
///
/// 1) wind
/// 2) length constraints
/// 3) capsule collisions
/// 4) update tangents
/// 5) write back to g_HairVertexPositions
pub struct TfxSim3Pass {
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl TfxSim3Pass {
  /// Change this in `_sim_common.glsl` too
  const THREAD_GROUP_SIZE: u32 = TfxSim0Pass::THREAD_GROUP_SIZE;

  const BINDING_INDEX_CONFIG_UBO: u32 = 0;
  const BINDING_INDEX_POSITIONS: u32 = 1;
  const BINDING_INDEX_POSITIONS_PREV: u32 = 2;
  const BINDING_INDEX_POSITIONS_INITIAL: u32 = 3;
  const BINDING_INDEX_TANGENTS: u32 = 4;

  pub fn new(vk_app: &VkCtx) -> Self {
    info!("Creating {}", get_simple_type_name::<Self>());
    let device = vk_app.vk_device();
    let pipeline_cache = &vk_app.pipeline_cache;

    let uniforms_desc = Self::get_uniforms_layout();
    let uniforms_layout = create_push_descriptor_layout(device, uniforms_desc);
    let pipeline_layout = create_pipeline_layout(device, &[uniforms_layout], &[]);
    let pipeline = create_compute_pipeline(device, pipeline_cache, &pipeline_layout, SHADER_PATH);

    Self {
      pipeline,
      pipeline_layout,
      uniforms_layout,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    device.destroy_descriptor_set_layout(self.uniforms_layout, None);
    device.destroy_pipeline_layout(self.pipeline_layout, None);
    device.destroy_pipeline(self.pipeline, None);
  }

  fn get_uniforms_layout() -> Vec<vk::DescriptorSetLayoutBinding> {
    vec![
      create_ubo_binding(
        Self::BINDING_INDEX_CONFIG_UBO,
        vk::ShaderStageFlags::COMPUTE,
      ),
      create_ssbo_binding(Self::BINDING_INDEX_POSITIONS, vk::ShaderStageFlags::COMPUTE),
      create_ssbo_binding(
        Self::BINDING_INDEX_POSITIONS_PREV,
        vk::ShaderStageFlags::COMPUTE,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_POSITIONS_INITIAL,
        vk::ShaderStageFlags::COMPUTE,
      ),
      create_ssbo_binding(Self::BINDING_INDEX_TANGENTS, vk::ShaderStageFlags::COMPUTE),
    ]
  }

  pub fn execute(&self, exec_ctx: &PassExecContext, entity: &TfxObject) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    let pass_name = &create_per_object_pass_name::<Self>(&entity.name);

    unsafe {
      let scope_id = exec_ctx.cmd_start_compute_pass(pass_name);
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        self.pipeline,
      );

      // bind uniforms
      self.bind_uniforms(exec_ctx, entity);

      // execute
      let group_count_x = group_count_x_per_vertex(entity, Self::THREAD_GROUP_SIZE);
      device.cmd_dispatch(command_buffer, group_count_x, 1, 1);

      // end
      exec_ctx.cmd_end_compute_pass(scope_id);
    }
  }

  unsafe fn bind_uniforms(&self, exec_ctx: &PassExecContext, entity: &TfxObject) {
    let frame_idx = exec_ctx.timer.frame_idx();
    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
    let [positions_current, positions_prev, _] = entity.get_position_buffers(frame_idx);
    let config_buffer = exec_ctx.config_buffer;

    let uniform_resouces = [
      BindableResource::Buffer {
        usage: BindableBufferUsage::UBO,
        binding: Self::BINDING_INDEX_CONFIG_UBO,
        buffer: config_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_POSITIONS,
        buffer: positions_current,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_POSITIONS_PREV,
        buffer: positions_prev,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_POSITIONS_INITIAL,
        buffer: &entity.initial_positions_buffer,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_TANGENTS,
        buffer: &entity.tangents_buffer,
      },
    ];
    bind_resources_to_descriptors_compute(&resouce_binder, 0, &uniform_resouces);
  }
}
