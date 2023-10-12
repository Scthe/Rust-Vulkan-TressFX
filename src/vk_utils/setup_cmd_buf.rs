use ash;
use ash::vk;

pub trait WithSetupCmdBuffer {
  fn with_setup_cb(&self, callback: impl FnOnce(&ash::Device, vk::CommandBuffer));
}

pub unsafe fn execute_setup_cmd_buf(
  device: &ash::Device,
  queue: vk::Queue,
  cmd_buf: vk::CommandBuffer,
  callback: impl FnOnce(&ash::Device, vk::CommandBuffer),
) {
  // begin setup
  let cmd_buf_begin_info = vk::CommandBufferBeginInfo::builder()
    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
    .build();
  device
    .begin_command_buffer(cmd_buf, &cmd_buf_begin_info)
    // also resets command buffer
    .expect("Failed - with_setup_cb:begin_command_buffer");

  // execute
  callback(device, cmd_buf);

  // end+submit
  device
    .end_command_buffer(cmd_buf)
    .expect("Failed - with_setup_cb:end_command_buffer()");
  let submit_info = vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cmd_buf));
  device
    .queue_submit(queue, &[submit_info.build()], vk::Fence::null())
    .expect("Failed with_setup_cb:queue_submit()");

  log::trace!("with_setup_cb: device_wait_idle");
  device
    .device_wait_idle()
    .expect("Failed with_setup_cb:device_wait_idle()");
}
