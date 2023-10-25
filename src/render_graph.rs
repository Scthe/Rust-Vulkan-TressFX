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
mod present_pass;
mod ssao_pass;
mod tonemapping_pass;

pub use self::_shared::*;
use self::forward_pass::{ForwardPass, ForwardPassFramebuffer};
use self::ssao_pass::{SSAOPass, SSAOPassFramebuffer};
use self::tonemapping_pass::{TonemappingPass, TonemappingPassFramebuffer};
use self::{_shared::GlobalConfigUBO, present_pass::PresentPass};

// TODO add check when compiling shader if .glsl is newer than .spv. Then panic and say to recompile shaders

/// https://github.com/Scthe/WebFX/blob/master/src/main.ts#L144
pub struct RenderGraph {
  /// Refreshed once every frame. Contains e.g. all config settings, camera data
  /// One per frame-in-flight.
  config_uniform_buffers: Vec<VkBuffer>,
  framebuffers: Vec<FrameFramebuffers>,

  // passes
  forward_pass: ForwardPass,
  ssao_pass: SSAOPass,
  tonemapping_pass: TonemappingPass,
  present_pass: PresentPass,
}

impl RenderGraph {
  pub fn new(vk_app: &VkCtx, config: &Config) -> Self {
    let image_format = vk_app.swapchain.surface_format.format;
    let in_flight_frames = vk_app.frames_in_flight();

    // scene uniform buffers - memory allocations + descriptor set
    let config_uniform_buffers = allocate_config_uniform_buffers(vk_app, in_flight_frames);

    // create passes
    let forward_pass = ForwardPass::new(vk_app);
    let ssao_pass = SSAOPass::new(vk_app);
    let tonemapping_pass = TonemappingPass::new(vk_app);
    let present_pass = PresentPass::new(vk_app, image_format);

    let mut render_graph = RenderGraph {
      config_uniform_buffers,
      framebuffers: Vec::with_capacity(in_flight_frames),
      forward_pass,
      ssao_pass,
      tonemapping_pass,
      present_pass,
    };

    // framebuffers
    info!("Creating framebuffers - one for each frame in flight");
    render_graph.initialize_framebuffers(vk_app, config);
    render_graph
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();

    // passes
    self.present_pass.destroy(device);
    self.tonemapping_pass.destroy(device);
    self.ssao_pass.destroy(vk_app);
    self.forward_pass.destroy(vk_app);

    // framebuffers
    self.framebuffers.iter_mut().for_each(|framebuffer| {
      device.destroy_framebuffer(framebuffer.present_pass, None);
      framebuffer.forward_pass.destroy(vk_app);
      framebuffer.ssao_pass.destroy(vk_app);
      framebuffer.tonemapping_pass.destroy(vk_app);
    });

    // uniform buffers
    let allocator = &vk_app.allocator;
    self.config_uniform_buffers.iter_mut().for_each(|buffer| {
      buffer.unmap_memory(allocator);
      buffer.delete(allocator);
    })
  }

  pub fn get_last_render_pass(&self) -> vk::RenderPass {
    self.present_pass.render_pass
  }

  pub fn execute_render_graph(
    &mut self,
    vk_app: &VkCtx,
    config: &mut Config,
    scene: &World,
    frame_idx: usize,
    app_ui: &mut AppUI,
    window: &winit::window::Window,
  ) {
    // 'heavy' ash's objects
    let device = vk_app.vk_device();
    let swapchain = &vk_app.swapchain;

    // 'light' vulkan objects (just pointers really)
    let queue = vk_app.device.queue;

    // per frame data so we can have many frames in processing at the same time
    // TODO instead of '%' operator, use `swapchain_image_index`?
    let frame_data = vk_app.data_per_frame(frame_idx % vk_app.frames_in_flight());
    let cmd_buf = frame_data.command_buffer;

    // get next swapchain image (view and framebuffer)
    let (swapchain_image_index, _) = unsafe {
      swapchain
        .swapchain_loader
        .acquire_next_image(
          swapchain.swapchain,
          u64::MAX,
          frame_data.present_complete_semaphore, // 'acquire_semaphore'
          vk::Fence::null(),
        )
        .expect("Failed to acquire next swapchain image")
    };

    // update per-frame uniforms
    let config_vk_buffer = &self.config_uniform_buffers[swapchain_image_index as usize];
    self.update_config_uniform_buffer(vk_app, config, scene, config_vk_buffer);
    self.update_model_uniform_buffers(scene, swapchain_image_index as usize);

    // sync between frames
    unsafe {
      device
        .wait_for_fences(&[frame_data.draw_command_fence], true, u64::MAX)
        .expect("vkWaitForFences at frame start failed");
      device
        .reset_fences(&[frame_data.draw_command_fence])
        .expect("vkResetFences at frame start failed");
    }

    // pass ctx
    let mut pass_ctx = PassExecContext {
      swapchain_image_idx: swapchain_image_index as usize,
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

    let framebuffers = &mut self.framebuffers[swapchain_image_index as usize];

    // forward rendering
    RenderGraph::debug_start_pass(&pass_ctx, "forward_pass");
    self
      .forward_pass
      .execute(&pass_ctx, &mut framebuffers.forward_pass);

    // ssao
    RenderGraph::debug_start_pass(&pass_ctx, "ssao_pass");
    self.ssao_pass.execute(
      &pass_ctx,
      &mut framebuffers.ssao_pass,
      &mut framebuffers.forward_pass.depth_stencil_tex,
      framebuffers.forward_pass.depth_image_view,
      &mut framebuffers.forward_pass.normals_tex,
    );

    // color grading + tonemapping
    RenderGraph::debug_start_pass(&pass_ctx, "tonemapping_pass");
    self.tonemapping_pass.execute(
      &pass_ctx,
      &mut framebuffers.tonemapping_pass,
      &mut framebuffers.forward_pass.diffuse_tex,
    );

    // final pass to render output to OS window framebuffer
    RenderGraph::debug_start_pass(&pass_ctx, "present_pass");
    self.present_pass.execute(
      &mut pass_ctx,
      &mut framebuffers.present_pass,
      app_ui,
      &mut framebuffers.tonemapping_pass.tonemapped_tex,
      &mut framebuffers.forward_pass.normals_tex,
      &mut framebuffers.ssao_pass.ssao_tex,
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
      .wait_semaphores(&[frame_data.present_complete_semaphore])
      .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
      .command_buffers(&[cmd_buf])
      .signal_semaphores(&[frame_data.rendering_complete_semaphore]) // release_semaphore
      .build();
    unsafe {
      device
        .queue_submit(queue, &[submit_info], frame_data.draw_command_fence)
        .expect("Failed queue_submit()");
    }

    // present queue result
    let present_info = vk::PresentInfoKHR::builder()
      .image_indices(&[swapchain_image_index])
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
  }

  fn debug_start_pass(exec_ctx: &PassExecContext, name: &str) {
    if exec_ctx.config.only_first_frame {
      info!("Start {}", name);
    }
  }

  fn update_config_uniform_buffer(
    &self,
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

  fn update_model_uniform_buffers(&self, scene: &World, frame_id: usize) {
    let camera = &scene.camera;
    scene.entities.iter().for_each(|entity| {
      let data = ForwardModelUBO::new(entity, camera);
      let data_bytes = bytemuck::bytes_of(&data);
      let buffer = entity.get_ubo_buffer(frame_id);
      buffer.write_to_mapped(data_bytes);
    });
  }

  fn initialize_framebuffers(&mut self, vk_app: &VkCtx, config: &Config) {
    let swapchain_image_views = &vk_app.swapchain.image_views;
    let window_size = &vk_app.window_size();

    swapchain_image_views
      .iter()
      .enumerate()
      .for_each(|(frame_id, &iv)| {
        let forward_pass = self
          .forward_pass
          .create_framebuffer(vk_app, frame_id, window_size);
        let ssao_pass = self.ssao_pass.create_framebuffer(vk_app, frame_id, config);
        let tonemapping_pass =
          self
            .tonemapping_pass
            .create_framebuffer(vk_app, frame_id, window_size);
        let present_pass = self
          .present_pass
          .create_framebuffer(vk_app, iv, window_size);

        self.framebuffers.push(FrameFramebuffers {
          forward_pass,
          ssao_pass,
          tonemapping_pass,
          present_pass,
        });
      });
  }
}

fn allocate_config_uniform_buffers(vk_app: &VkCtx, in_flight_frames: usize) -> Vec<VkBuffer> {
  let size = size_of::<GlobalConfigUBO>() as _;
  let usage = vk::BufferUsageFlags::UNIFORM_BUFFER;

  (0..in_flight_frames)
    .map(|i| {
      let allocator = &vk_app.allocator;
      let mut buffer = VkBuffer::empty(
        format!("scene_uniform_buffers_{}", i),
        size,
        usage,
        allocator,
        vk_app.device.queue_family_index,
        true,
      );
      buffer.map_memory(allocator); // always mapped
      buffer
    })
    .collect::<Vec<_>>()
}

struct FrameFramebuffers {
  forward_pass: ForwardPassFramebuffer,
  ssao_pass: SSAOPassFramebuffer,
  tonemapping_pass: TonemappingPassFramebuffer,
  present_pass: vk::Framebuffer,
}
