use std::mem::size_of;

use ash;
use ash::vk;
use bytemuck;

use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

mod _shared;
mod forward_pass;
mod present_pass;

pub use self::_shared::RenderableVertex;
use self::forward_pass::{ForwardPass, ForwardPassFramebuffer};
use self::{_shared::GlobalConfigUniformBuffer, present_pass::PresentPass};

// TODO add check when compiling shader if .glsl is newer than .spv. Then panic and say to recompile shaders

pub struct RenderGraph {
  /// Refreshed once every frame. Contains e.g. all config settings, camera data
  /// One per frame-in-flight.
  config_uniform_buffers: Vec<VkBuffer>,

  // ForwardPass
  forward_pass: ForwardPass,
  forward_pass_framebuffers: Vec<ForwardPassFramebuffer>,
  // PresentPass
  present_pass: PresentPass,
  present_pass_framebuffers: Vec<vk::Framebuffer>,
}

impl RenderGraph {
  pub fn new(vk_app: &VkCtx) -> Self {
    let image_format = vk_app.swapchain.surface_format.format;
    let swapchain_image_views = &vk_app.swapchain.image_views;
    let in_flight_frames = vk_app.frames_in_flight();

    // scene uniform buffers - memory allocations + descriptor set
    let config_uniform_buffers = allocate_config_uniform_buffers(vk_app, in_flight_frames);

    // create passes
    let forward_pass = ForwardPass::new(vk_app);
    let present_pass = PresentPass::new(vk_app, image_format);

    // framebuffers
    let window_size = &vk_app.window_size();
    let forward_pass_framebuffers = swapchain_image_views
      .iter()
      .map(|_| forward_pass.create_framebuffer(vk_app, window_size))
      .collect();
    let present_pass_framebuffers = swapchain_image_views
      .iter()
      .map(|&iv| present_pass.create_framebuffer(vk_app, iv, window_size))
      .collect();

    RenderGraph {
      config_uniform_buffers,
      forward_pass,
      forward_pass_framebuffers,
      present_pass,
      present_pass_framebuffers,
    }
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();

    // passes
    self.present_pass.destroy(device);
    self.forward_pass.destroy(device);

    // framebuffers
    self
      .present_pass_framebuffers
      .iter_mut()
      .for_each(|framebuffer| {
        device.destroy_framebuffer(*framebuffer, None);
      });
    self
      .forward_pass_framebuffers
      .iter_mut()
      .for_each(|framebuffer| {
        framebuffer.destroy(vk_app);
      });

    // uniform buffers
    let allocator = &vk_app.allocator;
    self.config_uniform_buffers.iter_mut().for_each(|buffer| {
      buffer.unmap_memory(allocator);
      buffer.delete(allocator);
    })
  }

  pub fn execute_render_graph(&self, vk_app: &VkCtx, scene: &World, frame_idx: usize) {
    // 'heavy' ash's objects
    let device = vk_app.vk_device();
    let swapchain = &vk_app.swapchain;

    // 'light' vulkan objects (just pointers really)
    let queue = vk_app.device.queue;

    // per frame data so we can have many frames in processing at the same time
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
    let framebuffers = self.framebuffers_for_swapchain_image(swapchain_image_index as usize);

    // update per-frame uniforms
    let config_vk_buffer = &self.config_uniform_buffers[swapchain_image_index as usize];
    self.update_scene_uniform_buffer(scene, config_vk_buffer);

    unsafe {
      device
        .wait_for_fences(&[frame_data.draw_command_fence], true, u64::MAX)
        .expect("vkWaitForFences at frame start failed");
      device
        .reset_fences(&[frame_data.draw_command_fence])
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

    /*
    self.forward_pass.execute(
      vk_app,
      scene,
      cmd_buf,
      framebuffers.forward_pass,
      swapchain.size,
      config_vk_buffer,
    );
    */

    self.present_pass.execute(
      vk_app,
      cmd_buf,
      framebuffers.present_pass,
      swapchain.size,
      // &framebuffers.forward_pass.diffuse_tex,
      &scene.test_texture,
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

  fn update_scene_uniform_buffer(&self, scene: &World, vk_buffer: &VkBuffer) {
    let camera = &scene.camera;
    let v = camera.view_matrix().clone(); // TODO clone?
    let p = camera.perspective_matrix().clone();
    let data = GlobalConfigUniformBuffer {
      u_vp: p.mul_mat4(&v),
      // u_mvp: p.mul_mat4(&v).mul_mat4(&m),
    };

    let data_bytes = bytemuck::bytes_of(&data);
    vk_buffer.write_to_mapped(data_bytes);
  }

  fn framebuffers_for_swapchain_image(&self, swapchain_image_index: usize) -> FrameFramebuffers {
    FrameFramebuffers {
      forward_pass: &self.forward_pass_framebuffers[swapchain_image_index],
      present_pass: &self.present_pass_framebuffers[swapchain_image_index],
    }
  }
}

fn allocate_config_uniform_buffers(vk_app: &VkCtx, in_flight_frames: usize) -> Vec<VkBuffer> {
  let size = size_of::<GlobalConfigUniformBuffer>() as _;
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

struct FrameFramebuffers<'a> {
  forward_pass: &'a ForwardPassFramebuffer,
  present_pass: &'a vk::Framebuffer,
}
