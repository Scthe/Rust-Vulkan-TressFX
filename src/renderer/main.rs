use log::trace;

use ash;
pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use crate::vk_app::AppVk;
use crate::vk_utils::resources::create_viewport;
use crate::vk_utils::swapchain::size_to_rect_vk;

fn cmd_draw_triangle(
  device: &ash::Device,
  command_buffer: vk::CommandBuffer,
  render_pass: vk::RenderPass,
  pipeline: vk::Pipeline,
  framebuffer: vk::Framebuffer,
  size: vk::Extent2D,
) -> () {
  let render_area = size_to_rect_vk(&size);
  let clear_color = vk::ClearColorValue {
    float32: [0.2f32, 0.2f32, 0.2f32, 1f32],
  };

  let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
    .render_pass(render_pass)
    .framebuffer(framebuffer)
    .render_area(render_area)
    .clear_values(&[vk::ClearValue { color: clear_color }])
    .build();

  trace!("Registering commands to draw triangle");
  unsafe {
    /*
    device.cmd_pipeline_barrier(
      *command_buffer,
      vk::PipelineStageFlags::ALL_GRAPHICS,
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      dependency_flags,
      memory_barriers,
      buffer_memory_barriers,
      image_memory_barriers
    );*/

    trace!("cmd_begin_render_pass");
    device.cmd_begin_render_pass(
      command_buffer,
      &render_pass_begin_info,
      vk::SubpassContents::INLINE,
    );

    // draw calls go here
    device.cmd_set_viewport(command_buffer, 0, &[create_viewport(&size)]);
    device.cmd_set_scissor(command_buffer, 0, &[render_area]);
    trace!("cmd_bind_pipeline");
    device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
    trace!("cmd_draw");
    device.cmd_draw(command_buffer, 3, 1, 0, 0);

    trace!("cmd_end_render_pass");
    device.cmd_end_render_pass(command_buffer)
  }
}

pub unsafe fn render_loop(vk_app: &AppVk) {
  let device = &vk_app.device.device;
  let swapchain = &vk_app.swapchain;
  let synchronize = &vk_app.synchronize;

  let cmd_buf = vk_app.command_buffers.cmd_buf_triangle;
  let queue = vk_app.device.queue;
  let render_pass = vk_app.render_passes.render_pass_triangle;
  let pipeline = vk_app.pipelines.pipeline_triangle;

  // get next swapchain image (view and framebuffer)
  let (swapchain_image_index, _) = swapchain
    .swapchain_loader
    .acquire_next_image(
      swapchain.swapchain,
      u64::MAX,
      synchronize.present_complete_semaphore, // 'acquire_semaphore'
      vk::Fence::null(),
    )
    .expect("Failed to acquire next swapchain image");

  let framebuffer = swapchain.framebuffers[swapchain_image_index as usize];

  device
    .wait_for_fences(&[synchronize.draw_commands_fence], true, u64::MAX)
    .unwrap();

  device
    .reset_fences(&[synchronize.draw_commands_fence])
    .unwrap();
  //
  // start record command buffer
  let cmd_buf_begin_info = vk::CommandBufferBeginInfo::builder()
  // can be one time submit bit for optimization
  .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
  .build();
  device
    .begin_command_buffer(cmd_buf, &cmd_buf_begin_info)
    .expect("Failed - begin_command_buffer");

  trace!("BEFORE cmd_draw_triangle()");
  cmd_draw_triangle(
    &device,
    cmd_buf,
    render_pass,
    pipeline,
    framebuffer,
    swapchain.size,
  );
  trace!("AFTER cmd_draw_triangle()");

  device
    .end_command_buffer(cmd_buf)
    .expect("Failed - end_command_buffer(");
  // end record command buffer
  //

  // submit command buffers to the queue
  let submit_info = vk::SubmitInfo::builder()
    .wait_semaphores(&[synchronize.present_complete_semaphore])
    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
    .command_buffers(&[cmd_buf])
    .signal_semaphores(&[synchronize.rendering_complete_semaphore]) // release_semaphore
    .build();
  device
    .queue_submit(queue, &[submit_info], synchronize.draw_commands_fence)
    .expect("Failed queue_submit()");

  // present queue result
  let present_info = vk::PresentInfoKHR::builder()
    .image_indices(&[swapchain_image_index])
    // .results(results) // p_results: ptr::null_mut(),
    .swapchains(&[swapchain.swapchain])
    .wait_semaphores(&[synchronize.rendering_complete_semaphore])
    .build();

  swapchain
    .swapchain_loader
    .queue_present(queue, &present_info)
    .expect("Failed queue_present()");
}
