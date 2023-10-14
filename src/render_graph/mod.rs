use std::mem::size_of;

use ash;
use ash::vk;
use bytemuck;

use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

mod _shared;
mod forward_pass;

use self::_shared::GlobalConfigUniformBuffer;
pub use self::_shared::RenderableVertex;
use self::forward_pass::ForwardPass;

// TODO add check when compiling shader if .glsl is newer than .spv. Then panic and say to recompile shaders

pub struct RenderGraph {
  forward_pass: ForwardPass,
  framebuffers: Vec<vk::Framebuffer>,

  /// Refreshed once every frame. Contains e.g. all config settings, camera data
  /// One per frame-in-flight.
  config_uniform_buffers: Vec<VkBuffer>,
}

impl RenderGraph {
  pub fn new(vk_app: &VkCtx) -> Self {
    let image_format = vk_app.swapchain.surface_format.format;
    let swapchain_image_views = &vk_app.swapchain.image_views;
    let in_flight_frames = vk_app.frames_in_flight();

    // scene uniform buffers - memory allocations + descriptor set
    let config_uniform_buffers = allocate_config_uniform_buffers(vk_app, in_flight_frames);

    // create passes
    let forward_pass = ForwardPass::new(vk_app, image_format);

    // framebuffers
    let device = vk_app.vk_device();
    let window_size = &vk_app.window_size();
    let framebuffers = create_framebuffers(
      device,
      forward_pass.render_pass,
      swapchain_image_views,
      window_size,
    );

    RenderGraph {
      forward_pass,
      framebuffers,
      config_uniform_buffers,
    }
  }

  pub unsafe fn destroy(&mut self, vk_app: &VkCtx) {
    let device = vk_app.vk_device();

    // passes
    self.forward_pass.destroy(device);

    // framebuffers
    for &framebuffer in &self.framebuffers {
      device.destroy_framebuffer(framebuffer, None);
    }

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
    let framebuffer = self.framebuffer_for_swapchain_image(swapchain_image_index);

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

    self.forward_pass.execute(
      vk_app,
      scene,
      cmd_buf,
      framebuffer,
      swapchain.size,
      config_vk_buffer,
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
      // TODO or reverse? p*v*m
      // u_vp: p, // TODO or reverse?
      u_vp: p.mul_mat4(&v),
    };

    let data_bytes = bytemuck::bytes_of(&data);
    vk_buffer.write_to_mapped(data_bytes);
  }

  fn framebuffer_for_swapchain_image(&self, swapchain_image_index: u32) -> vk::Framebuffer {
    self.framebuffers[swapchain_image_index as usize]
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
