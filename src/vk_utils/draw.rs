use ash;
use ash::vk;

use super::{create_viewport, size_to_rect_vk};

pub unsafe fn cmd_begin_render_pass_for_framebuffer(
  device: &ash::Device,
  command_buffer: &vk::CommandBuffer,
  render_pass: &vk::RenderPass,
  framebuffer: &vk::Framebuffer,
  viewport_size: &vk::Extent2D,
  clear_values: &[vk::ClearValue],
) {
  let render_area = size_to_rect_vk(&viewport_size);
  let viewport = create_viewport(&viewport_size);

  let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
    .render_pass(*render_pass)
    .framebuffer(*framebuffer)
    .render_area(render_area)
    .clear_values(clear_values)
    .build();

  // record commands
  device.cmd_begin_render_pass(
    *command_buffer,
    &render_pass_begin_info,
    vk::SubpassContents::INLINE,
  );
  device.cmd_set_viewport(*command_buffer, 0, &[viewport]);
  device.cmd_set_scissor(*command_buffer, 0, &[render_area]);
}

pub unsafe fn cmd_draw_fullscreen_triangle(
  device: &ash::Device,
  command_buffer: &vk::CommandBuffer,
) {
  // 3 vertices (1 triangle), 1 instance, no special offset
  device.cmd_draw(*command_buffer, 3, 1, 0, 0);
}
