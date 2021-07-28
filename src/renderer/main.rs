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
  let viewport = create_viewport(&size);
  let clear_color = vk::ClearColorValue {
    float32: [0.2f32, 0.2f32, 0.2f32, 1f32],
  };

  let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
    .render_pass(render_pass)
    .framebuffer(framebuffer)
    .render_area(render_area)
    .clear_values(&[vk::ClearValue { color: clear_color }])
    .build();

  unsafe {
    /*
    device.cmd_pipeline_barrier(
      *command_buffer,
      vk::PipelineStageFlags::ALL_GRAPHICS,
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      dependency_flags,
      memory_barriers,
      buffer_memory_barriers,
      image_memory_barriers,
    );
    */

    device.cmd_begin_render_pass(
      command_buffer,
      &render_pass_begin_info,
      vk::SubpassContents::INLINE,
    );

    // draw calls go here
    device.cmd_set_viewport(command_buffer, 0, &[viewport]);
    device.cmd_set_scissor(command_buffer, 0, &[render_area]);
    device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
    device.cmd_draw(command_buffer, 3, 1, 0, 0);

    device.cmd_end_render_pass(command_buffer)
  }
}

pub fn render_loop(vk_app: &AppVk) {
  let device = &vk_app.device.device;
  let swapchain = &vk_app.swapchain;
  let synchronize = &vk_app.synchronize;

  // let cmd_buf = vk_app.command_buffers.cmd_buf_triangle; // TODO one per frame-in-flight
  let queue = vk_app.device.queue;
  let render_pass = vk_app.render_passes.render_pass_triangle;
  let pipeline = vk_app.pipelines.pipeline_triangle;

  /*
  if (command_buffer_submitted[current_command_buffer])
    {
      err = vkWaitForFences(vulkan_globals.device, 1, &command_buffer_fences[current_command_buffer], VK_TRUE, UINT64_MAX);
      if (err != VK_SUCCESS)
        Sys_Error("vkWaitForFences failed");
    }

    err = vkResetFences(vulkan_globals.device, 1, &command_buffer_fences[current_command_buffer]);
    if (err != VK_SUCCESS)
      Sys_Error("vkResetFences failed");
  */

  // get next swapchain image (view and framebuffer)
  let (swapchain_image_index, _) = unsafe {
    swapchain
      .swapchain_loader
      .acquire_next_image(
        swapchain.swapchain,
        u64::MAX,
        synchronize.present_complete_semaphore, // 'acquire_semaphore'
        vk::Fence::null(),
      )
      .expect("Failed to acquire next swapchain image")
  };
  let frame_data = vk_app.data_per_swapchain_image(swapchain_image_index as usize);
  let cmd_buf = frame_data.command_buffer;
  // println!("swapchain_image_index={}", swapchain_image_index);

  unsafe {
    device
      .wait_for_fences(&[frame_data.draw_command_fence], true, u64::MAX)
      .unwrap();
    device
      .reset_fences(&[frame_data.draw_command_fence])
      .unwrap();
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

  cmd_draw_triangle(
    &device,
    cmd_buf,
    render_pass,
    pipeline,
    frame_data.framebuffer,
    swapchain.size,
  );

  unsafe {
    device
      .end_command_buffer(cmd_buf)
      .expect("Failed - end_command_buffer(");
  }
  // end record command buffer
  //

  // submit command buffers to the queue
  let submit_info = vk::SubmitInfo::builder()
    .wait_semaphores(&[synchronize.present_complete_semaphore])
    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
    .command_buffers(&[cmd_buf])
    .signal_semaphores(&[synchronize.rendering_complete_semaphore]) // release_semaphore
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
    .wait_semaphores(&[synchronize.rendering_complete_semaphore])
    .build();

  unsafe {
    swapchain
      .swapchain_loader
      .queue_present(queue, &present_info)
      .expect("Failed queue_present()");
  }
}
