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
  resources_per_frame: Vec<PerFrameResources>,

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
    let in_flight_frames = vk_app.frames_in_flight();

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
      resources_per_frame: Vec::with_capacity(in_flight_frames),
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

    render_graph.initialize_per_frame_resources(vk_app, config);
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

    // per frame resources
    self.resources_per_frame.iter_mut().for_each(|res| {
      res.destroy(vk_app);
    });
  }

  pub fn execute_render_graph(
    &mut self,
    window: &winit::window::Window,
    vk_app: &VkCtx,
    frame_idx: usize,
    config: &mut Config,
    scene: &mut World,
    app_ui: &mut AppUI,
    timer: &AppTimer,
    profiler: &mut GpuProfiler,
  ) {
    // 'heavy' ash's objects
    let device = vk_app.vk_device();
    let swapchain = &vk_app.swapchain;

    // 'light' vulkan objects (just pointers really)
    let queue = vk_app.device.queue;

    // Per frame data so we can have many frames in processing at the same time.
    // All of this is for synchronization between the in-flight-frames.
    // For anything else, use `swapchain_image_index` as it is connected to particular
    // OS-window framebuffer, which in turn has passess/barriers connecting it to other
    // per-frame resources.
    let frame_sync = vk_app.data_per_frame(frame_idx % vk_app.frames_in_flight());
    let cmd_buf = frame_sync.command_buffer;

    // get next swapchain image (view and framebuffer)
    // https://themaister.net/blog/2023/11/12/my-scuffed-game-streaming-adventure-pyrofling/
    let swapchain_image_index: usize = unsafe {
      swapchain
        .swapchain_loader
        .acquire_next_image(
          swapchain.swapchain,
          u64::MAX,
          frame_sync.swapchain_image_acquired_semaphore, // 'acquire_semaphore'
          vk::Fence::null(),
        )
        .expect("Failed to acquire next swapchain image")
        .0 as _
    };

    // update per-frame uniforms
    let frame_resources = &mut self.resources_per_frame[swapchain_image_index];
    let config_vk_buffer = &frame_resources.config_uniform_buffer;
    update_config_uniform_buffer(vk_app, config, timer, scene, config_vk_buffer);
    update_model_uniform_buffers(config, scene, swapchain_image_index);
    update_tfx_uniform_buffers(config, scene, swapchain_image_index);

    // sync between frames
    // since we have usually 3 frames in flight, wait for queue submit
    // of the frame that was 3 frames before.
    // TODO [HIGH] shouldn't this be before we "update per-frame uniforms"?
    unsafe {
      device
        .wait_for_fences(&[frame_sync.queue_submit_finished_fence], true, u64::MAX)
        .expect("vkWaitForFences at frame start failed");
      device
        .reset_fences(&[frame_sync.queue_submit_finished_fence])
        .expect("vkResetFences at frame start failed");
    }

    //
    // start record command buffer
    let cmd_buf_begin_info = vk::CommandBufferBeginInfo::builder()
      // can be one time submit bit for optimization We will rerecord cmds before next submit
      .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
      .build();
    unsafe {
      device
      .begin_command_buffer(cmd_buf, &cmd_buf_begin_info) // also resets command buffer
      .expect("Failed - begin_command_buffer");
    }

    // execute render graph passes
    profiler.begin_frame(device, cmd_buf);

    // pass ctx
    let mut pass_ctx = PassExecContext {
      swapchain_image_idx: swapchain_image_index,
      vk_app,
      config,
      scene,
      command_buffer: cmd_buf,
      size: vk_app.window_size(),
      config_buffer: config_vk_buffer,
      window,
      timer,
      profiler: RefCell::new(profiler),
    };

    // simulate
    execute_tfx_simulation(&pass_ctx, &self.tfx_sim0, &self.tfx_sim2, &self.tfx_sim3);

    // shadow map generate pass
    self.shadow_map_pass.execute::<ShadowMapPass>(
      &pass_ctx,
      &mut frame_resources.shadow_map_pass,
      &pass_ctx.config.shadows.shadow_source,
      true,
    );

    // sss forward scatter depth map generate pass
    self.sss_depth_pass.execute(
      &pass_ctx,
      &mut frame_resources.sss_depth_pass,
      &self.shadow_map_pass,
      &pass_ctx.config.sss_forward_scatter.source,
    );

    // forward rendering
    self.forward_pass.execute(
      &pass_ctx,
      &mut frame_resources.forward_pass,
      &mut frame_resources.shadow_map_pass.depth_tex,
      &mut frame_resources.sss_depth_pass.depth_tex,
      &mut frame_resources.ssao_pass.ssao_tex,
    );

    // linear depth
    self.linear_depth_pass.execute(
      &pass_ctx,
      &mut frame_resources.linear_depth_pass,
      &mut frame_resources.forward_pass.depth_stencil_tex,
      frame_resources.forward_pass.depth_image_view,
    );

    // sss blur
    // skip SSSBlur pass for special debug modes
    if !pass_ctx.config.preserve_original_forward_pass_result() {
      self.sss_blur_pass.execute(
        &pass_ctx,
        &mut frame_resources.sss_blur_fbo0,
        &mut frame_resources.sss_blur_fbo1,
        &mut frame_resources.forward_pass.diffuse_tex, // 1st read, 2nd write
        &mut frame_resources.sss_ping_result_tex,      // 1st write, 2nd read
        &mut frame_resources.forward_pass.depth_stencil_tex, // write (stencil source)
        &mut frame_resources.linear_depth_pass.linear_depth_tex, // read
      );
    }

    // Render hair
    // we have to do it after SSS, as it would create depth discontinuities
    // that are hard to get rid off. Since this pass writes to depth buffer,
    // we have to update linear depth render target too
    if pass_ctx.config.is_hair_using_ppll() {
      execute_tfx_ppll(
        &self.tfx_ppll_build_pass,
        &self.tfx_ppll_resolve_pass,
        &self.tfx_depth_only_pass,
        &pass_ctx,
        &mut frame_resources.tfx_ppll_build_pass,
        &mut frame_resources.tfx_ppll_resolve_pass,
        frame_resources.tfx_depth_only_pass,
        &mut frame_resources.forward_pass.depth_stencil_tex,
        &mut frame_resources.forward_pass.diffuse_tex,
        &mut frame_resources.ssao_pass.ssao_tex,
        &mut frame_resources.shadow_map_pass.depth_tex,
      );
    } else {
      self.tfx_forward_pass.execute(
        &pass_ctx,
        &mut frame_resources.forward_pass,
        &mut frame_resources.shadow_map_pass.depth_tex,
        &mut frame_resources.ssao_pass.ssao_tex,
      );
    }

    // linear depth again, after hair has written to original depth buffer
    self.linear_depth_pass.execute(
      &pass_ctx,
      &mut frame_resources.linear_depth_pass,
      &mut frame_resources.forward_pass.depth_stencil_tex,
      frame_resources.forward_pass.depth_image_view,
    );

    // ssao
    self.ssao_pass.execute(
      &pass_ctx,
      &mut frame_resources.ssao_pass,
      &mut frame_resources.forward_pass.depth_stencil_tex,
      frame_resources.forward_pass.depth_image_view,
      &mut frame_resources.forward_pass.normals_tex,
    );

    // ssao blur
    self.ssao_blur_pass.execute(
      &pass_ctx,
      "SSAO",
      &mut frame_resources.ssao_blur_fbo0,
      &mut frame_resources.ssao_blur_fbo1,
      &mut frame_resources.ssao_pass.ssao_tex,
      &mut frame_resources.ssao_ping_result_tex,
      pass_ctx.config.get_ssao_viewport_size(),
      &mut frame_resources.linear_depth_pass.linear_depth_tex,
      pass_ctx.config.ssao.blur_radius,
      pass_ctx.config.ssao.blur_max_depth_distance,
      pass_ctx.config.ssao.blur_gauss_sigma,
    );

    // color grading + tonemapping
    self.tonemapping_pass.execute(
      &pass_ctx,
      &mut frame_resources.tonemapping_pass,
      &mut frame_resources.forward_pass.diffuse_tex,
    );

    // final pass to render output to OS window framebuffer
    self.present_pass.execute(
      &mut pass_ctx,
      &mut frame_resources.present_pass,
      app_ui,
      &mut frame_resources.forward_pass.diffuse_tex,
      &mut frame_resources.tonemapping_pass.tonemapped_tex,
      &mut frame_resources.forward_pass.normals_tex,
      &mut frame_resources.ssao_pass.ssao_tex,
      &mut frame_resources.forward_pass.depth_stencil_tex,
      frame_resources.forward_pass.depth_image_view,
      &mut frame_resources.shadow_map_pass.depth_tex,
      &mut frame_resources.linear_depth_pass.linear_depth_tex,
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
      .wait_semaphores(&[frame_sync.swapchain_image_acquired_semaphore])
      .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
      .command_buffers(&[cmd_buf])
      .signal_semaphores(&[frame_sync.queue_submit_finished_semaphore]) // release_semaphore
      .build();
    unsafe {
      device
        .queue_submit(
          queue,
          &[submit_info],
          frame_sync.queue_submit_finished_fence,
        )
        .expect("Failed queue_submit()");
    }

    // present queue result after queue finished work
    let present_info = vk::PresentInfoKHR::builder()
      .image_indices(&[swapchain_image_index as _])
      // .results(results) // p_results: ptr::null_mut(),
      .swapchains(&[swapchain.swapchain])
      .wait_semaphores(&[frame_sync.queue_submit_finished_semaphore])
      .build();

    unsafe {
      swapchain
        .swapchain_loader
        .queue_present(queue, &present_info)
        .expect("Failed queue_present()");
    }

    profiler.end_frame(device);
  }

  fn initialize_per_frame_resources(&mut self, vk_app: &VkCtx, config: &Config) {
    let swapchain_image_views = &vk_app.swapchain.image_views;
    let window_size = &vk_app.window_size();

    swapchain_image_views
      .iter()
      .enumerate()
      .for_each(|(frame_id, &iv)| {
        let ssao_result_size = config.get_ssao_viewport_size();

        // textures
        let sss_ping_result_tex = ForwardPass::create_diffuse_attachment_tex::<SSSBlurPass>(
          vk_app,
          "sss_blur_tmp",
          frame_id,
          window_size,
        );
        let ssao_ping_result_tex =
          SSAOPass::create_result_texture(vk_app, &ssao_result_size, frame_id, true);

        // fbos
        let shadow_map_pass = self.shadow_map_pass.create_framebuffer::<ShadowMapPass>(
          vk_app,
          frame_id,
          config.shadows.shadowmap_size,
        );
        let sss_depth_pass = self.sss_depth_pass.create_framebuffer(
          vk_app,
          frame_id,
          &self.shadow_map_pass,
          config.sss_forward_scatter.depthmap_size,
        );
        let forward_pass = self
          .forward_pass
          .create_framebuffer(vk_app, frame_id, window_size);
        let tfx_ppll_build_pass = self.tfx_ppll_build_pass.create_framebuffer(
          vk_app,
          frame_id,
          &forward_pass.depth_stencil_tex,
        );
        let tfx_ppll_resolve_pass = self.tfx_ppll_resolve_pass.create_framebuffer(
          vk_app,
          &forward_pass.depth_stencil_tex,
          &forward_pass.diffuse_tex,
          &forward_pass.normals_tex,
        );
        let tfx_depth_only_pass = self
          .tfx_depth_only_pass
          .create_framebuffer(vk_app, &forward_pass.depth_stencil_tex);
        let sss_blur_fbo0 = self.sss_blur_pass.create_framebuffer(
          vk_app,
          &forward_pass.depth_stencil_tex,
          &sss_ping_result_tex,
        );
        let sss_blur_fbo1 = self.sss_blur_pass.create_framebuffer(
          vk_app,
          &forward_pass.depth_stencil_tex,
          &forward_pass.diffuse_tex,
        );
        let linear_depth_pass =
          self
            .linear_depth_pass
            .create_framebuffer(vk_app, frame_id, window_size);
        let ssao_pass = self
          .ssao_pass
          .create_framebuffer(vk_app, frame_id, &ssao_result_size);
        let ssao_blur_fbo0 = self
          .ssao_blur_pass
          .create_framebuffer(vk_app, &ssao_ping_result_tex);
        let ssao_blur_fbo1 = self
          .ssao_blur_pass
          .create_framebuffer(vk_app, &ssao_pass.ssao_tex);
        let tonemapping_pass =
          self
            .tonemapping_pass
            .create_framebuffer(vk_app, frame_id, window_size);
        let present_pass = self
          .present_pass
          .create_framebuffer(vk_app, iv, window_size);

        // config buffer
        let config_uniform_buffer = allocate_config_uniform_buffer(vk_app, frame_id);

        self.resources_per_frame.push(PerFrameResources {
          config_uniform_buffer,
          // fbos
          shadow_map_pass,
          sss_depth_pass,
          sss_blur_fbo0,
          sss_blur_fbo1,
          forward_pass,
          tfx_ppll_build_pass,
          tfx_ppll_resolve_pass,
          tfx_depth_only_pass,
          linear_depth_pass,
          ssao_pass,
          ssao_blur_fbo0,
          ssao_blur_fbo1,
          tonemapping_pass,
          present_pass,
          // textures
          sss_ping_result_tex,
          ssao_ping_result_tex,
        });
      });

    transition_window_framebuffers_for_present_khr(vk_app);
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
    VkBufferMemoryPreference::Mappable,
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

fn update_model_uniform_buffers(config: &Config, scene: &World, frame_id: usize) {
  let camera = &scene.camera;
  scene.entities.iter().for_each(|entity| {
    entity.update_ubo_data(frame_id, config, camera);
  });
}

fn update_tfx_uniform_buffers(config: &Config, scene: &World, frame_id: usize) {
  scene.tressfx_objects.iter().for_each(|entity| {
    entity.update_params_uniform_buffer(frame_id, config);
  });
}
