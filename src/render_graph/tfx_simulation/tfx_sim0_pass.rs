use ash;
use ash::vk;
use log::info;

use crate::scene::TfxObject;
use crate::utils::get_simple_type_name;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

use super::PassExecContext;

const SHADER_PATH: &str =
  "./assets/shaders-compiled/sim0_IntegrationAndGlobalShapeConstraints.comp.spv";

/// Verlet integration for gravity/static forces (but not wind)
/// and global shape constraints.
pub struct TfxSim0Pass {
  pipeline: vk::Pipeline,
  pipeline_layout: vk::PipelineLayout,
  uniforms_layout: vk::DescriptorSetLayout,
}

impl TfxSim0Pass {
  /// Change this in `_sim_common.glsl` too
  const THREAD_GROUP_SIZE: u32 = 64;

  const BINDING_INDEX_POSITIONS: u32 = 0;
  const BINDING_INDEX_POSITIONS_PREV: u32 = 1;
  const BINDING_INDEX_POSITIONS_PREV_PREV: u32 = 2;
  const BINDING_INDEX_POSITIONS_INITIAL: u32 = 3;

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
      create_ssbo_binding(Self::BINDING_INDEX_POSITIONS, vk::ShaderStageFlags::COMPUTE),
      create_ssbo_binding(
        Self::BINDING_INDEX_POSITIONS_PREV,
        vk::ShaderStageFlags::COMPUTE,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_POSITIONS_PREV_PREV,
        vk::ShaderStageFlags::COMPUTE,
      ),
      create_ssbo_binding(
        Self::BINDING_INDEX_POSITIONS_INITIAL,
        vk::ShaderStageFlags::COMPUTE,
      ),
    ]
  }

  pub fn execute(&self, exec_ctx: &PassExecContext, entity: &TfxObject) -> () {
    let vk_app = exec_ctx.vk_app;
    let command_buffer = exec_ctx.command_buffer;
    let device = vk_app.vk_device();
    // let pass_name = &get_simple_type_name::<Self>();

    unsafe {
      execute_full_pipeline_barrier(device, command_buffer); // TODO better barriers! https://vulkan-tutorial.com/Compute_Shader#page_Synchronizing-graphics-and-compute

      // TODO add to exec_ctx, just like `exec_ctx.cmd_start_render_pass` to add profiling etc.
      device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::COMPUTE,
        self.pipeline,
      );

      // bind uniforms
      self.bind_uniforms(exec_ctx, entity);

      // execute
      // https://github.com/Scthe/TressFX-OpenGL/blob/master/libs/amd_tressfx/src/TressFXSimulation.cpp#L51
      let group_count_x = entity.vertex_count() / Self::THREAD_GROUP_SIZE;
      device.cmd_dispatch(command_buffer, group_count_x, 1, 1);

      execute_full_pipeline_barrier(device, command_buffer); // TODO better barriers! https://vulkan-tutorial.com/Compute_Shader#page_Synchronizing-graphics-and-compute
    }
  }

  unsafe fn bind_uniforms(&self, exec_ctx: &PassExecContext, entity: &TfxObject) {
    let frame_idx = exec_ctx.timer.frame_idx();
    let resouce_binder = exec_ctx.create_resouce_binder(self.pipeline_layout);
    let [positions_current, positions_prev, positions_prev_prev] =
      entity.get_position_buffers(frame_idx);

    let uniform_resouces = [
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
        binding: Self::BINDING_INDEX_POSITIONS_PREV_PREV,
        buffer: positions_prev_prev,
      },
      BindableResource::Buffer {
        usage: BindableBufferUsage::SSBO,
        binding: Self::BINDING_INDEX_POSITIONS_INITIAL,
        buffer: &entity.initial_positions_buffer,
      },
    ];
    bind_resources_to_descriptors_compute(&resouce_binder, 0, &uniform_resouces);
  }
}
