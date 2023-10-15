use ash;
use ash::vk;
use log::trace;

#[allow(dead_code)]
pub fn create_framebuffers_with_one_attachment(
  device: &ash::Device,
  render_pass: vk::RenderPass,
  // TODO should be Vec<Vec<vk::ImageView>> Each framebuffer has an array of attachments.
  // ATM this only allows one vk::ImageView per framebuffer
  image_views: &Vec<vk::ImageView>,
  size: &vk::Extent2D,
) -> Vec<vk::Framebuffer> {
  trace!("Will create {} framebuffers {:?}", image_views.len(), size);
  image_views
    .iter()
    .map(|&iv| create_framebuffer(device, render_pass, &[iv], size))
    .collect()
}

pub fn create_framebuffer(
  device: &ash::Device,
  render_pass: vk::RenderPass,
  image_views: &[vk::ImageView],
  size: &vk::Extent2D,
) -> vk::Framebuffer {
  // trace!("Will create framebuffer {}x{}", size.width, size.height);

  let create_info = vk::FramebufferCreateInfo::builder()
    .render_pass(render_pass)
    .attachments(image_views)
    .width(size.width)
    .height(size.height)
    .layers(1)
    .build();
  unsafe {
    device
      .create_framebuffer(&create_info, None)
      .expect("Failed to create framebuffer")
  }
}
