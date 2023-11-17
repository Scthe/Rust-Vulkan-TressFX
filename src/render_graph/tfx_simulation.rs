mod tfx_sim0_pass;
mod tfx_sim2_pass;

use ash::vk;

use crate::{scene::TfxObject, vk_utils::execute_full_pipeline_barrier};

pub use self::tfx_sim0_pass::*;
pub use self::tfx_sim2_pass::*;

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
pub fn execute_tfx_simulation(
  pass_ctx: &PassExecContext,
  tfx_sim0: &TfxSim0Pass,
  tfx_sim2: &TfxSim2Pass,
) {
  let scene = &*pass_ctx.scene;
  for entity in &scene.tressfx_objects {
    cmd_barrier_prepare_for_simulation(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);

    tfx_sim0.execute(pass_ctx, entity);

    cmd_barrier_between_simulation_steps(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);

    let local_shape_iterations = 1; // TODO hardcoded
    for _ in 0..local_shape_iterations {
      tfx_sim2.execute(pass_ctx, entity);
      cmd_barrier_between_simulation_steps(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);
    }

    cmd_barrier_prepare_for_render(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);
  }
}

fn cmd_barrier_prepare_for_simulation(device: &ash::Device, command_buffer: vk::CommandBuffer) {
  // TODO better barriers! https://vulkan-tutorial.com/Compute_Shader#page_Synchronizing-graphics-and-compute
  unsafe { execute_full_pipeline_barrier(device, command_buffer) };
}

fn cmd_barrier_prepare_for_render(device: &ash::Device, command_buffer: vk::CommandBuffer) {
  // TODO better barriers! https://vulkan-tutorial.com/Compute_Shader#page_Synchronizing-graphics-and-compute
  unsafe { execute_full_pipeline_barrier(device, command_buffer) };
}

fn cmd_barrier_between_simulation_steps(device: &ash::Device, command_buffer: vk::CommandBuffer) {
  // TODO better barriers! https://vulkan-tutorial.com/Compute_Shader#page_Synchronizing-graphics-and-compute
  //      We should also consider which step changes which buffers.
  //      e.g. local constraints sim2 only requires current+initial,
  //      so no barrier for _prev, _prev_prev after it.
  unsafe { execute_full_pipeline_barrier(device, command_buffer) };
}
