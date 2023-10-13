use ash;
use ash::vk;
use log::trace;

pub fn create_framebuffers(
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

fn create_framebuffer(
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
  let framebuffer = unsafe {
    device
      .create_framebuffer(&create_info, None)
      .expect("Failed to create framebuffer")
  };
  // trace!("Framebuffer created");

  framebuffer
}
