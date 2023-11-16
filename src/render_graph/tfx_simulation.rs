mod tfx_sim0_pass;

use ash::vk;

use crate::{scene::TfxObject, vk_utils::execute_full_pipeline_barrier};

pub use self::tfx_sim0_pass::*;

use super::PassExecContext;

/// https://github.com/Scthe/TressFX-OpenGL/blob/master/libs/amd_tressfx/src/TressFXSimulation.cpp#L51
pub fn group_count_x_per_vertex(entity: &TfxObject, thread_group_size: u32) -> u32 {
  entity.vertex_count() / thread_group_size
}

pub fn execute_tfx_simulation(pass_ctx: &PassExecContext, tfx_sim0: &TfxSim0Pass) {
  let scene = &*pass_ctx.scene;
  for entity in &scene.tressfx_objects {
    cmd_barrier_prepare_for_simulation(pass_ctx.vk_app.vk_device(), pass_ctx.command_buffer);

    tfx_sim0.execute(pass_ctx, entity);

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
