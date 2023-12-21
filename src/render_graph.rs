use std::cell::RefCell;
use std::mem::size_of;

use ash;
use ash::vk;
use bytemuck;

use crate::app_timer::AppTimer;
use crate::app_ui::AppUI;
use crate::config::Config;
use crate::gpu_profiler::GpuProfiler;
use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

mod _shared;
mod blur_pass;
mod forward_pass;
mod linear_depth_pass;
mod present_pass;
mod shadow_map_pass;
mod ssao_pass;
mod sss_blur_pass;
mod sss_depth_pass;
mod tfx_render;
mod tfx_simulation;
mod tonemapping_pass;

pub use self::_shared::*;
use self::blur_pass::BlurPass;
use self::forward_pass::ForwardPass;
use self::linear_depth_pass::LinearDepthPass;
use self::shadow_map_pass::ShadowMapPass;
use self::ssao_pass::SSAOPass;
use self::sss_blur_pass::SSSBlurPass;
use self::sss_depth_pass::SSSDepthPass;
use self::tfx_render::{
  execute_tfx_ppll, TfxDepthOnlyPass, TfxForwardPass, TfxPpllBuildPass, TfxPpllResolvePass,
};
use self::tfx_simulation::{execute_tfx_simulation, TfxSim0Pass, TfxSim2Pass, TfxSim3Pass};
use self::tonemapping_pass::TonemappingPass;
use self::{_shared::GlobalConfigUBO, present_pass::PresentPass};

// TODO [IGNORE] add check when compiling shader if .glsl is newer than .spv. Then panic and say to recompile shaders

/// https://github.com/Scthe/WebFX/blob/master/src/main.ts#L144
pub struct RenderGraph {
  /// 1 per frame-in-flight
  per_frame_data: Vec<FrameData>,
  /// 1 per swapchain image
  present_fbos: Vec<vk::Framebuffer>,
  rg_resources: Option<RenderGraphResources>,

  // passes
  shadow_map_pass: ShadowMapPass,
  sss_depth_pass: SSSDepthPass,
  sss_blur_pass: SSSBlurPass,
  forward_pass: ForwardPass,
  tfx_forward_pass: TfxForwardPass,
  tfx_ppll_build_pass: TfxPpllBuildPass,
  tfx_ppll_resolve_pass: TfxPpllResolvePass,
  tfx_depth_only_pass: TfxDepthOnlyPass,
  tfx_sim0: TfxSim0Pass,
  tfx_sim2: TfxSim2Pass,
  tfx_sim3: TfxSim3Pass,
  linear_depth_pass: LinearDepthPass,
  ssao_pass: SSAOPass,
  ssao_blur_pass: BlurPass,
  tonemapping_pass: TonemappingPass,
  present_pass: PresentPass,
}

impl RenderGraph {
  pub fn new(vk_app: &VkCtx, config: &Config) -> Self {
    let image_format = vk_app.swapchain.surface_format.format;

    // create passes
    let shadow_map_pass = ShadowMapPass::new(vk_app);
    let sss_depth_pass = SSSDepthPass::new();
    let sss_blur_pass = SSSBlurPass::new(vk_app);
    let forward_pass = ForwardPass::new(vk_app);
    let linear_depth_pass = LinearDepthPass::new(vk_app);
    let ssao_pass = SSAOPass::new(vk_app);
    let ssao_blur_pass = BlurPass::new(vk_app, SSAOPass::RESULT_TEXTURE_FORMAT);
    let tfx_forward_pass = TfxForwardPass::new(vk_app);
    let tfx_ppll_build_pass = TfxPpllBuildPass::new(vk_app);
    let tfx_ppll_resolve_pass = TfxPpllResolvePass::new(vk_app);
    let tfx_depth_only_pass = TfxDepthOnlyPass::new(vk_app);
    let tfx_sim0 = TfxSim0Pass::new(vk_app);
    let tfx_sim2 = TfxSim2Pass::new(vk_app);
    let tfx_sim3 = TfxSim3Pass::new(vk_app);
    let tonemapping_pass = TonemappingPass::new(vk_app);
    let present_pass = PresentPass::new(vk_app, image_format);

    let mut render_graph = RenderGraph {
      per_frame_data: Vec::with_capacity(config.frames_in_flight),
      present_fbos: Vec::with_capacity(vk_app.swapchain_images_count()),
      rg_resources: None,
      shadow_map_pass,
      sss_depth_pass,
      sss_blur_pass,
      forward_pass,
      tfx_forward_pass,
      tfx_ppll_build_pass,
      tfx_ppll_resolve_pass,
      tfx_depth_only_pass,
      tfx_sim0,
      tfx_sim2,
      tfx_sim3,
      linear_depth_pass,
      ssao_pass,
      ssao_blur_pass,
      tonemapping_pass,
      present_pass,
    };

    render_graph.rg_resources = Some(RenderGraphResources::new(vk_app, config, &render_graph));
    render_graph.initialize_per_frame_resources(vk_app, config);
    render_graph.initialize_per_swapchain_img_resources(vk_app);
    render_graph
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();

    // passes
    self.present_pass.destroy(device);
    self.tonemapping_pass.destroy(device);
    self.ssao_pass.destroy(vk_app);
    self.ssao_blur_pass.destroy(device);
    self.linear_depth_pass.destroy(device);
    self.tfx_forward_pass.destroy(vk_app);
    self.tfx_ppll_build_pass.destroy(vk_app);
    self.tfx_ppll_resolve_pass.destroy(vk_app);
    self.tfx_depth_only_pass.destroy(device);
    self.tfx_sim0.destroy(device);
    self.tfx_sim2.destroy(device);
    self.tfx_sim3.destroy(device);
    self.forward_pass.destroy(vk_app);
    self.sss_depth_pass.destroy();
    self.sss_blur_pass.destroy(device);
    self.shadow_map_pass.destroy(device);

    self.rg_resources.as_mut().map(|res| res.destroy(vk_app));

    // per frame resources
    self.per_frame_data.iter_mut().for_each(|res| {
      res.destroy(vk_app);
    });
    // per swapchain resources
    self.present_fbos.iter_mut().for_each(|res| {
      device.destroy_framebuffer(*res, None);
    });
  }

  pub fn execute_render_graph(
    &mut self,
    window: &winit::window::Window,
    vk_app: &VkCtx,
    config: &mut Config,
    scene: &mut World,
    app_ui: &mut AppUI,
    timer: &AppTimer,
    profiler: &mut GpuProfiler,
  ) {
    let device = vk_app.vk_device();
    let swapchain = &vk_app.swapchain;
    let queue = vk_app.device.queue;
    let frame_idx = timer.frame_idx();
    let frame_in_flight_id: FrameInFlightId = (frame_idx % (config.frames_in_flight as u64)) as _;

    // sync between frames
    let frame_data = &self.per_frame_data[frame_in_flight_id];
    self.wait_for_previous_frame_in_flight(vk_app, frame_data);

    // update per-frame uniforms
    let config_vk_buffer = &frame_data.config_uniform_buffer;
    update_config_uniform_buffer(vk_app, config, timer, scene, config_vk_buffer);
    update_model_uniform_buffers(config, scene, frame_in_flight_id);
    update_tfx_uniform_buffers(config, scene, frame_in_flight_id);

    // acquire next swapchain image
    let swapchain_image = vk_app.acquire_next_swapchain_image(frame_data);

    //
    // start record command buffer
    let cmd_buf = frame_data.command_buffer;
    begin_command_buffer_for_one_time_submit(device, cmd_buf);

    // execute render graph passes
    profiler.begin_frame(device, cmd_buf);

    // pass ctx
    let pass_ctx = PassExecContext {
      frame_in_flight_id,
      vk_app,
      config: RefCell::new(config),
      scene: RefCell::new(scene),
      command_buffer: cmd_buf,
      size: vk_app.window_size(),
      config_buffer: config_vk_buffer,
      window,
      timer,
      profiler: RefCell::new(profiler),
    };
    let res = &mut self
      .rg_resources
      .as_mut()
      .expect("RenderGraph resources were not initialized before starting to render a frame");

    // simulate
    execute_tfx_simulation(&pass_ctx, &self.tfx_sim0, &self.tfx_sim2, &self.tfx_sim3);

    // shadow map generate pass
    self.shadow_map_pass.execute::<ShadowMapPass>(
      &pass_ctx,
      &mut res.shadow_map_pass,
      &pass_ctx.config.borrow().shadows.shadow_source,
      true,
    );

    // sss forward scatter depth map generate pass
    self.sss_depth_pass.execute(
      &pass_ctx,
      &mut res.sss_depth_pass,
      &self.shadow_map_pass,
      &pass_ctx.config.borrow().sss_forward_scatter.source,
    );

    // forward rendering
    self.forward_pass.execute(
      &pass_ctx,
      &mut res.forward_pass,
      &mut res.shadow_map_pass.depth_tex,
      &mut res.sss_depth_pass.depth_tex,
      &mut res.ssao_pass.ssao_tex,
    );

    // linear depth
    self.linear_depth_pass.execute(
      &pass_ctx,
      &mut res.linear_depth_pass,
      &mut res.forward_pass.depth_stencil_tex,
      res.forward_pass.depth_image_view,
    );

    // sss blur
    // skip SSSBlur pass for special debug modes
    if !pass_ctx
      .config
      .borrow()
      .preserve_original_forward_pass_result()
    {
      self.sss_blur_pass.execute(
        &pass_ctx,
        &mut res.sss_blur_fbo0,
        &mut res.sss_blur_fbo1,
        &mut res.forward_pass.diffuse_tex, // 1st read, 2nd write
        &mut res.sss_ping_result_tex,      // 1st write, 2nd read
        &mut res.forward_pass.depth_stencil_tex, // write (stencil source)
        &mut res.linear_depth_pass.linear_depth_tex, // read
      );
    }

    // Render hair
    // we have to do it after SSS, as it would create depth discontinuities
    // that are hard to get rid off. Since this pass writes to depth buffer,
    // we have to update linear depth render target too
    if pass_ctx.config.borrow().is_hair_using_ppll() {
      execute_tfx_ppll(
        &self.tfx_ppll_build_pass,
        &self.tfx_ppll_resolve_pass,
        &self.tfx_depth_only_pass,
        &pass_ctx,
        &mut res.tfx_ppll_build_pass,
        &mut res.tfx_ppll_resolve_pass,
        res.tfx_depth_only_pass,
        &mut res.forward_pass.depth_stencil_tex,
        &mut res.forward_pass.diffuse_tex,
        &mut res.ssao_pass.ssao_tex,
        &mut res.shadow_map_pass.depth_tex,
      );
    } else {
      self.tfx_forward_pass.execute(
        &pass_ctx,
        &mut res.forward_pass,
        &mut res.shadow_map_pass.depth_tex,
        &mut res.ssao_pass.ssao_tex,
      );
    }

    // linear depth again, after hair has written to original depth buffer
    self.linear_depth_pass.execute(
      &pass_ctx,
      &mut res.linear_depth_pass,
      &mut res.forward_pass.depth_stencil_tex,
      res.forward_pass.depth_image_view,
    );

    // ssao
    self.ssao_pass.execute(
      &pass_ctx,
      &mut res.ssao_pass,
      &mut res.forward_pass.depth_stencil_tex,
      res.forward_pass.depth_image_view,
      &mut res.forward_pass.normals_tex,
    );

    // ssao blur
    self.ssao_blur_pass.execute(
      &pass_ctx,
      "SSAO",
      &mut res.ssao_blur_fbo0,
      &mut res.ssao_blur_fbo1,
      &mut res.ssao_pass.ssao_tex,
      &mut res.ssao_ping_result_tex,
      pass_ctx.config.borrow().get_ssao_viewport_size(),
      &mut res.linear_depth_pass.linear_depth_tex,
      pass_ctx.config.borrow().ssao.blur_radius,
      pass_ctx.config.borrow().ssao.blur_max_depth_distance,
      pass_ctx.config.borrow().ssao.blur_gauss_sigma,
    );

    // color grading + tonemapping
    self.tonemapping_pass.execute(
      &pass_ctx,
      &mut res.tonemapping_pass,
      &mut res.forward_pass.diffuse_tex,
    );

    // final pass to render output to OS window framebuffer
    let present_fbo = self.present_fbos[swapchain_image.index];
    self.present_pass.execute(
      &pass_ctx,
      &present_fbo,
      app_ui,
      &mut res.forward_pass.diffuse_tex,
      &mut res.tonemapping_pass.tonemapped_tex,
      &mut res.forward_pass.normals_tex,
      &mut res.ssao_pass.ssao_tex,
      &mut res.forward_pass.depth_stencil_tex,
      res.forward_pass.depth_image_view,
      &mut res.shadow_map_pass.depth_tex,
      &mut res.linear_depth_pass.linear_depth_tex,
    );

    unsafe {
      device
        .end_command_buffer(cmd_buf)
        .expect("Failed - end_command_buffer()");
    }
    // end record command buffer
    //

    // submit command buffers to the queue
    let submit_info = vk::SubmitInfo::builder()
      .wait_semaphores(&[frame_data.acquire_semaphore])
      .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
      .command_buffers(&[cmd_buf])
      .signal_semaphores(&[frame_data.rendering_complete_semaphore])
      .build();
    unsafe {
      device
        .queue_submit(
          queue,
          &[submit_info],
          frame_data.queue_submit_finished_fence,
        )
        .expect("Failed queue_submit()");
    }

    // present queue result after queue finished work
    let present_info = vk::PresentInfoKHR::builder()
      .image_indices(&[swapchain_image.index as _])
      // .results(results) // p_results: ptr::null_mut(),
      .swapchains(&[swapchain.swapchain])
      .wait_semaphores(&[frame_data.rendering_complete_semaphore])
      .build();

    unsafe {
      swapchain
        .swapchain_loader
        .queue_present(queue, &present_info)
        .expect("Failed queue_present()");
    }

    profiler.end_frame(device);
  }

  /// `vkWaitForFences`
  fn wait_for_previous_frame_in_flight(&self, vk_app: &VkCtx, frame_data: &FrameData) {
    let device = vk_app.vk_device();
    unsafe {
      device
        .wait_for_fences(&[frame_data.queue_submit_finished_fence], true, u64::MAX)
        .expect("vkWaitForFences at frame start failed");
      device
        .reset_fences(&[frame_data.queue_submit_finished_fence])
        .expect("vkResetFences at frame start failed");
    }
  }

  /// initialize resources used per frame-in-flight
  fn initialize_per_frame_resources(&mut self, vk_app: &VkCtx, config: &Config) {
    let frames_in_flight = config.frames_in_flight;

    (0..frames_in_flight).for_each(|frame_id| {
      let config_uniform_buffer = allocate_config_uniform_buffer(vk_app, frame_id);
      self
        .per_frame_data
        .push(FrameData::new(vk_app, config_uniform_buffer));
    });
  }

  /// initialize resources used per swapchain image
  fn initialize_per_swapchain_img_resources(&mut self, vk_app: &VkCtx) {
    let window_size = &vk_app.window_size();

    let barriers = vk_app
      .swapchain_images
      .iter()
      .map(|swapchain_image| {
        // create fbo
        let fbo =
          self
            .present_pass
            .create_framebuffer(vk_app, swapchain_image.image_view, window_size);
        self.present_fbos.push(fbo);

        // create barrier
        swapchain_image.create_barrier_transition_to_present_layout()
      })
      .collect::<Vec<_>>();

    // do the transitions
    vk_app.with_setup_cb(|device, cmd_buf| {
      let dep = vk::DependencyInfo::builder().image_memory_barriers(&barriers);
      unsafe { device.cmd_pipeline_barrier2(cmd_buf, &dep) };
    });
  }

  pub fn get_ui_draw_render_pass(&self) -> vk::RenderPass {
    self.present_pass.render_pass
  }
}

fn allocate_config_uniform_buffer(vk_app: &VkCtx, frame_id: usize) -> VkBuffer {
  let size = size_of::<GlobalConfigUBO>() as _;
  vk_app.create_buffer_empty(
    format!("scene_uniform_buffers_{}", frame_id),
    size,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
    VkMemoryPreference::GpuMappable,
  )
}

fn update_config_uniform_buffer(
  vk_app: &VkCtx,
  config: &Config,
  timer: &AppTimer,
  scene: &World,
  vk_buffer: &VkBuffer,
) {
  let camera = &scene.camera;
  let data = GlobalConfigUBO::new(vk_app, config, timer, camera);
  let data_bytes = bytemuck::bytes_of(&data);
  vk_buffer.write_to_mapped(data_bytes);
}

fn update_model_uniform_buffers(
  config: &Config,
  scene: &World,
  frame_in_flight_id: FrameInFlightId,
) {
  let camera = &scene.camera;
  scene.entities.iter().for_each(|entity| {
    entity.update_ubo_data(frame_in_flight_id, config, camera);
  });
}

fn update_tfx_uniform_buffers(config: &Config, scene: &World, frame_in_flight_id: FrameInFlightId) {
  scene.tressfx_objects.iter().for_each(|entity| {
    entity.update_params_uniform_buffer(frame_in_flight_id, config);
  });
}
