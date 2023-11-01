use std::mem::size_of;

use ash;
use ash::vk;
use bytemuck;
use log::info;

use crate::app_ui::AppUI;
use crate::config::Config;
use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

mod _shared;
mod forward_pass;
mod linear_depth_pass;
mod present_pass;
mod shadow_map_pass;
mod ssao_pass;
mod sss_blur_pass;
mod sss_depth_pass;
mod tonemapping_pass;

pub use self::_shared::*;
use self::forward_pass::ForwardPass;
use self::linear_depth_pass::LinearDepthPass;
use self::shadow_map_pass::ShadowMapPass;
use self::ssao_pass::SSAOPass;
use self::sss_blur_pass::SSSBlurPass;
use self::sss_depth_pass::SSSDepthPass;
use self::tonemapping_pass::TonemappingPass;
use self::{_shared::GlobalConfigUBO, present_pass::PresentPass};

// TODO add check when compiling shader if .glsl is newer than .spv. Then panic and say to recompile shaders

/// https://github.com/Scthe/WebFX/blob/master/src/main.ts#L144
pub struct RenderGraph {
  resources_per_frame: Vec<PerFrameResources>,

  // passes
  shadow_map_pass: ShadowMapPass,
  sss_depth_pass: SSSDepthPass,
  sss_blur_pass: SSSBlurPass,
  forward_pass: ForwardPass,
  linear_depth_pass: LinearDepthPass,
  ssao_pass: SSAOPass,
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
    let tonemapping_pass = TonemappingPass::new(vk_app);
    let present_pass = PresentPass::new(vk_app, image_format);

    let mut render_graph = RenderGraph {
      resources_per_frame: Vec::with_capacity(in_flight_frames),
      shadow_map_pass,
      sss_depth_pass,
      sss_blur_pass,
      forward_pass,
      linear_depth_pass,
      ssao_pass,
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
    self.linear_depth_pass.destroy(device);
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
    vk_app: &VkCtx,
    config: &mut Config,
    scene: &mut World,
    frame_idx: usize,
    app_ui: &mut AppUI,
    window: &winit::window::Window,
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
    let swapchain_image_index: usize = unsafe {
      swapchain
        .swapchain_loader
        .acquire_next_image(
          swapchain.swapchain,
          u64::MAX,
          frame_sync.present_complete_semaphore, // 'acquire_semaphore'
          vk::Fence::null(),
        )
        .expect("Failed to acquire next swapchain image")
        .0 as _
    };

    // update per-frame uniforms
    let frame_resources = &mut self.resources_per_frame[swapchain_image_index];
    let config_vk_buffer = &frame_resources.config_uniform_buffer;
    update_config_uniform_buffer(vk_app, config, scene, config_vk_buffer);
    update_model_uniform_buffers(config, scene, swapchain_image_index);

    // sync between frames
    unsafe {
      device
        .wait_for_fences(&[frame_sync.draw_command_fence], true, u64::MAX)
        .expect("vkWaitForFences at frame start failed");
      device
        .reset_fences(&[frame_sync.draw_command_fence])
        .expect("vkResetFences at frame start failed");
    }

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
    };

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

    // shadow map generate pass
    RenderGraph::debug_start_pass(&pass_ctx, "shadow_map_pass");
    self.shadow_map_pass.execute(
      &pass_ctx,
      &mut frame_resources.shadow_map_pass,
      &pass_ctx.config.shadows.shadow_source,
    );

    // sss forward scatter depth map generate pass
    RenderGraph::debug_start_pass(&pass_ctx, "sss_depth_pass");
    self.sss_depth_pass.execute(
      &pass_ctx,
      &mut frame_resources.sss_depth_pass,
      &self.shadow_map_pass,
      &pass_ctx.config.sss_forward_scatter.source,
    );

    // forward rendering
    RenderGraph::debug_start_pass(&pass_ctx, "forward_pass");
    self.forward_pass.execute(
      &pass_ctx,
      &mut frame_resources.forward_pass,
      &mut frame_resources.shadow_map_pass.depth_tex,
      &mut frame_resources.sss_depth_pass.depth_tex,
    );

    // linear depth
    RenderGraph::debug_start_pass(&pass_ctx, "linear_depth_pass");
    self.linear_depth_pass.execute(
      &pass_ctx,
      &mut frame_resources.linear_depth_pass,
      &mut frame_resources.forward_pass.depth_stencil_tex,
      frame_resources.forward_pass.depth_image_view,
    );

    // sss blur
    // skip SSSBlur pass for special debug modes
    if !pass_ctx.config.preserve_original_forward_pass_result() {
      RenderGraph::debug_start_pass(&pass_ctx, "sss_blur_0");
      self.sss_blur_pass.execute(
        &pass_ctx,
        &mut frame_resources.sss_blur_fbo0,
        SSSBlurPass::BLUR_DIRECTION_PASS0,
        &mut frame_resources.sss_ping_result_tex, // write
        &mut frame_resources.forward_pass.depth_stencil_tex, // write (stencil source)
        &mut frame_resources.forward_pass.diffuse_tex, // read
        &mut frame_resources.linear_depth_pass.linear_depth_tex, // read
      );

      RenderGraph::debug_start_pass(&pass_ctx, "sss_blur_1");
      self.sss_blur_pass.execute(
        &pass_ctx,
        &mut frame_resources.sss_blur_fbo1,
        SSSBlurPass::BLUR_DIRECTION_PASS1,
        &mut frame_resources.forward_pass.diffuse_tex, // write
        &mut frame_resources.forward_pass.depth_stencil_tex, // write (stencil source)
        &mut frame_resources.sss_ping_result_tex,      // read
        &mut frame_resources.linear_depth_pass.linear_depth_tex, // read
      );
    }

    // ssao
    RenderGraph::debug_start_pass(&pass_ctx, "ssao_pass");
    self.ssao_pass.execute(
      &pass_ctx,
      &mut frame_resources.ssao_pass,
      &mut frame_resources.forward_pass.depth_stencil_tex,
      frame_resources.forward_pass.depth_image_view,
      &mut frame_resources.forward_pass.normals_tex,
    );

    // color grading + tonemapping
    RenderGraph::debug_start_pass(&pass_ctx, "tonemapping_pass");
    self.tonemapping_pass.execute(
      &pass_ctx,
      &mut frame_resources.tonemapping_pass,
      &mut frame_resources.forward_pass.diffuse_tex,
    );

    // final pass to render output to OS window framebuffer
    RenderGraph::debug_start_pass(&pass_ctx, "present_pass");
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
      .wait_semaphores(&[frame_sync.present_complete_semaphore])
      .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
      .command_buffers(&[cmd_buf])
      .signal_semaphores(&[frame_sync.rendering_complete_semaphore]) // release_semaphore
      .build();
    unsafe {
      device
        .queue_submit(queue, &[submit_info], frame_sync.draw_command_fence)
        .expect("Failed queue_submit()");
    }

    // present queue result
    let present_info = vk::PresentInfoKHR::builder()
      .image_indices(&[swapchain_image_index as _])
      // .results(results) // p_results: ptr::null_mut(),
      .swapchains(&[swapchain.swapchain])
      .wait_semaphores(&[frame_sync.rendering_complete_semaphore])
      .build();

    unsafe {
      swapchain
        .swapchain_loader
        .queue_present(queue, &present_info)
        .expect("Failed queue_present()");
    }
  }

  fn initialize_per_frame_resources(&mut self, vk_app: &VkCtx, config: &Config) {
    let swapchain_image_views = &vk_app.swapchain.image_views;
    let window_size = &vk_app.window_size();

    swapchain_image_views
      .iter()
      .enumerate()
      .for_each(|(frame_id, &iv)| {
        let sss_ping_result_tex = ForwardPass::create_diffuse_attachment_tex(
          vk_app,
          window_size,
          format!("SSSBlurPass.pingResult#{}", frame_id),
        );

        let shadow_map_pass =
          self
            .shadow_map_pass
            .create_framebuffer(vk_app, frame_id, config.shadows.shadowmap_size);
        let sss_depth_pass = self.sss_depth_pass.create_framebuffer(
          vk_app,
          frame_id,
          &self.shadow_map_pass,
          config.sss_forward_scatter.depthmap_size,
        );
        let forward_pass = self
          .forward_pass
          .create_framebuffer(vk_app, frame_id, window_size);
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
        let ssao_pass = self.ssao_pass.create_framebuffer(vk_app, frame_id, config);
        let tonemapping_pass =
          self
            .tonemapping_pass
            .create_framebuffer(vk_app, frame_id, window_size);
        let present_pass = self
          .present_pass
          .create_framebuffer(vk_app, iv, window_size);

        let config_uniform_buffer = allocate_config_uniform_buffer(vk_app, frame_id);

        self.resources_per_frame.push(PerFrameResources {
          config_uniform_buffer,
          shadow_map_pass,
          sss_depth_pass,
          sss_blur_fbo0,
          sss_blur_fbo1,
          forward_pass,
          linear_depth_pass,
          ssao_pass,
          tonemapping_pass,
          present_pass,
          sss_ping_result_tex,
        });
      });

    transition_window_framebuffers_for_present_khr(vk_app);
  }

  fn debug_start_pass(exec_ctx: &PassExecContext, name: &str) {
    if exec_ctx.config.only_first_frame {
      info!("Start {}", name);
    }
  }

  pub fn get_ui_draw_render_pass(&self) -> vk::RenderPass {
    self.present_pass.render_pass
  }
}

fn allocate_config_uniform_buffer(vk_app: &VkCtx, frame_id: usize) -> VkBuffer {
  let size = size_of::<GlobalConfigUBO>() as _;
  let allocator = &vk_app.allocator;
  let mut buffer = VkBuffer::empty(
    format!("scene_uniform_buffers_{}", frame_id),
    size,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
    allocator,
    vk_app.device.queue_family_index,
    true,
  );
  buffer.map_memory(allocator); // always mapped
  buffer
}

fn update_config_uniform_buffer(
  vk_app: &VkCtx,
  config: &Config,
  scene: &World,
  vk_buffer: &VkBuffer,
) {
  let camera = &scene.camera;
  let data = GlobalConfigUBO::new(vk_app, config, camera);
  let data_bytes = bytemuck::bytes_of(&data);
  vk_buffer.write_to_mapped(data_bytes);
}

fn update_model_uniform_buffers(config: &Config, scene: &World, frame_id: usize) {
  let camera = &scene.camera;
  scene.entities.iter().for_each(|entity| {
    let data = ForwardModelUBO::new(config, entity, camera);
    let data_bytes = bytemuck::bytes_of(&data);
    let buffer = entity.get_ubo_buffer(frame_id);
    buffer.write_to_mapped(data_bytes);
  });
}
