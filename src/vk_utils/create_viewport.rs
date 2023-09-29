use ash::vk;

pub fn create_viewport(size: &vk::Extent2D) -> vk::Viewport {
  vk::Viewport {
    x: 0f32,
    y: size.height as f32, // flip vulkan coord system - important!
    width: size.width as f32,
    height: -(size.height as f32), // flip vulkan coord system - important!
    min_depth: 0f32,
    max_depth: 1.0f32,
    ..Default::default()
  }
}
