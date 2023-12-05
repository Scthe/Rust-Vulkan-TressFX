mod tfx_sim0_pass;
mod tfx_sim2_pass;
mod tfx_sim3_pass;

use ash::vk;

use crate::scene::TfxObject;
use crate::vk_utils::{cmd_storage_resource_barrier, VkStorageResourceBarrier};

pub use self::tfx_sim0_pass::*;
pub use self::tfx_sim2_pass::*;
pub use self::tfx_sim3_pass::*;

use super::PassExecContext;

/// https://github.com/Scthe/TressFX-OpenGL/blob/master/libs/amd_tressfx/src/TressFXSimulation.cpp#L51
pub fn group_count_x_per_vertex(entity: &TfxObject, thread_group_size: u32) -> u32 {
  entity.vertex_count() / thread_group_size
}

/// https://github.com/Scthe/TressFX-OpenGL/blob/master/libs/amd_tressfx/src/TressFXSimulation.cpp#L51
pub fn group_count_x_per_strand(entity: &TfxObject, thread_group_size: u32) -> u32 {
  entity.num_hair_strands / thread_group_size
}

/// https://github.com/Scthe/TressFX-OpenGL/blob/master/libs/amd_tressfx/src/TressFXSimulation.cpp#L51
/// https://github.com/Scthe/TressFX-OpenGL/blob/master/src/gl-tfx/TFxSimulation.cpp
pub fn execute_tfx_simulation(
  pass_ctx: &PassExecContext,
  tfx_sim0: &TfxSim0Pass,
  tfx_sim2: &TfxSim2Pass,
  tfx_sim3: &TfxSim3Pass,
) {
  let scene = pass_ctx.scene.borrow();
  let local_shape_iterations = pass_ctx
    .config
    .borrow()
    .tfx_simulation
    .local_stiffness_iterations;

  for entity in &scene.tressfx_objects {
    cmd_barrier_prepare_for_simulation(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);

    tfx_sim0.execute(pass_ctx, entity);

    cmd_barrier_between_simulation_steps(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);

    for _ in 0..local_shape_iterations {
      tfx_sim2.execute(pass_ctx, entity);
      cmd_barrier_between_simulation_steps(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);
    }

    tfx_sim3.execute(pass_ctx, entity);

    cmd_barrier_prepare_for_render(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);
  }
}

fn cmd_barrier_prepare_for_simulation(device: &ash::Device, command_buffer: vk::CommandBuffer) {
  unsafe {
    let barrier = VkStorageResourceBarrier {
      previous_op: (
        vk::PipelineStageFlags2::FRAGMENT_SHADER,
        vk::AccessFlags2::SHADER_READ,
      ),
      next_op: (
        vk::PipelineStageFlags2::COMPUTE_SHADER,
        vk::AccessFlags2::SHADER_WRITE | vk::AccessFlags2::SHADER_READ,
      ),
    };
    cmd_storage_resource_barrier(device, command_buffer, barrier);
  }
}

/// https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples#dispatch-writes-into-a-storage-buffer-draw-consumes-that-buffer-as-an-index-buffer
/// Though we use this not as an index buffer but SSBO.
fn cmd_barrier_prepare_for_render(device: &ash::Device, command_buffer: vk::CommandBuffer) {
  unsafe {
    let barrier = VkStorageResourceBarrier {
      previous_op: (
        vk::PipelineStageFlags2::COMPUTE_SHADER,
        vk::AccessFlags2::SHADER_WRITE,
      ),
      next_op: (
        vk::PipelineStageFlags2::VERTEX_SHADER,
        vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::MEMORY_READ,
      ),
    };
    cmd_storage_resource_barrier(device, command_buffer, barrier);
  }
}

/// https://github.com/KhronosGroup/Vulkan-Docs/wiki/Synchronization-Examples#three-dispatches-first-dispatch-writes-to-one-storage-buffer-second-dispatch-writes-to-a-different-storage-buffer-third-dispatch-reads-both
/// It says that global memory barriers are more effective here than per-resource.
fn cmd_barrier_between_simulation_steps(device: &ash::Device, command_buffer: vk::CommandBuffer) {
  unsafe {
    let barrier = VkStorageResourceBarrier {
      previous_op: (
        vk::PipelineStageFlags2::COMPUTE_SHADER,
        vk::AccessFlags2::SHADER_WRITE,
      ),
      next_op: (
        vk::PipelineStageFlags2::COMPUTE_SHADER,
        vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
      ),
    };
    cmd_storage_resource_barrier(device, command_buffer, barrier);
  }
}
