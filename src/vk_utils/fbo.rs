use log::trace;

use ash;
use ash::version::DeviceV1_0;
use ash::vk;

pub fn create_framebuffer(
  device: &ash::Device,
  render_pass: vk::RenderPass,
  image_views: &[vk::ImageView],
  size: &vk::Extent2D,
) -> vk::Framebuffer {
  trace!("Will create framebuffer {}x{}", size.width, size.height);

  let create_info = vk::FramebufferCreateInfo::builder()
    .render_pass(render_pass)
    .attachments(image_views)
    .width(size.width)
    .height(size.height)
    .layers(1)
    .build();
  let framebuffer = unsafe {
    device
      .create_framebuffer(&create_info, None)
      .expect("Failed to create framebuffer")
  };
  trace!("Framebuffer created");

  framebuffer
}
