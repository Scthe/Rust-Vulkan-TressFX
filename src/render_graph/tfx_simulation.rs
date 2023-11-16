mod tfx_sim0_pass;

pub use self::tfx_sim0_pass::*;

use super::PassExecContext;

pub fn execute_tfx_simulation(pass_ctx: &PassExecContext, tfx_sim0: &TfxSim0Pass) {
  let scene = &*pass_ctx.scene;
  for entity in &scene.tressfx_objects {
    tfx_sim0.execute(pass_ctx, entity);
  }
}
