use ash;
use ash::version::DeviceV1_0;
use ash::vk;

use crate::scene::World;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::create_framebuffer;

mod forward_pass;
use self::forward_pass::ForwardPass;
pub use self::forward_pass::TriangleVertex;

pub struct RenderGraph {
  forward_pass: ForwardPass,
  framebuffers: Vec<vk::Framebuffer>,
}

impl RenderGraph {
  pub fn new(vk_app: &VkCtx) -> Self {
    let image_format = vk_app.swapchain.surface_format.format;
    let forward_pass = ForwardPass::new(vk_app, image_format);

    // framebuffers
    let swapchain_image_views = &vk_app.swapchain.image_views;
    let framebuffers = swapchain_image_views
      .iter()
      .map(|&iv| {
        create_framebuffer(
          &vk_app.device.device,
          forward_pass.render_pass,
          &[iv],
          &vk_app.swapchain.size,
        )
      })
      .collect();

    RenderGraph {
      forward_pass,
      framebuffers,
    }
  }

  pub unsafe fn destroy(&self, device: &ash::Device) {
    self.forward_pass.destroy(device);

    for &framebuffer in &self.framebuffers {
      device.destroy_framebuffer(framebuffer, None);
    }
  }

  pub fn execute_render_graph(&self, vk_app: &VkCtx, scene: &World, frame_idx: usize) {
    // 'heavy' ash's objects
    let device = &vk_app.device.device;
    let swapchain = &vk_app.swapchain;

    // 'light' vulkan objects (just pointers really)
    let queue = vk_app.device.queue;
    // let render_pass = vk_app.render_passes.render_pass_triangle;
    // let pipeline = vk_app.pipelines.pipeline_triangle;
    // let vertex_buffer = scene.triangle_vertex_buffer.buffer;

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

    self
      .forward_pass
      .execute(&device, cmd_buf, &scene, framebuffer, swapchain.size);

    unsafe {
      device
        .end_command_buffer(cmd_buf)
        .expect("Failed - end_command_buffer(");
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

  fn framebuffer_for_swapchain_image(&self, swapchain_image_index: u32) -> vk::Framebuffer {
    self.framebuffers[swapchain_image_index as usize]
  }
}
