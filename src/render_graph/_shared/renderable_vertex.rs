use ash;
use ash::vk;
use bytemuck;
use glam::{Vec2, Vec3};

/// Represents vertex layout that is used for each renderable object in the scene.
/// Rendered in `ForwardPass`
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct RenderableVertex {
  /// position in 3d space
  pub position: Vec3,
  /// normalized normal vector for this vertex
  pub normal: Vec3,
  /// uv texture coordinates
  pub uv: Vec2,
}
unsafe impl bytemuck::Zeroable for RenderableVertex {}
unsafe impl bytemuck::Pod for RenderableVertex {}

impl RenderableVertex {
  #[allow(dead_code)]
  pub fn new(pos: (f32, f32, f32), n: (f32, f32, f32), uv: (f32, f32)) -> RenderableVertex {
    RenderableVertex {
      position: Vec3::new(pos.0, pos.1, pos.2),
      normal: Vec3::new(n.0, n.1, n.2),
      uv: Vec2::new(uv.0, uv.1),
    }
  }

  pub fn get_bindings_descriptions() -> [vk::VertexInputBindingDescription; 1] {
    [vk::VertexInputBindingDescription {
      binding: 0,
      input_rate: vk::VertexInputRate::VERTEX,
      stride: std::mem::size_of::<RenderableVertex>() as u32,
    }]
  }

  pub fn get_attributes_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
    [
      // position
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 0,
        format: vk::Format::R32G32B32_SFLOAT,
        offset: 0, // offsetof(RenderableVertex, pos),
      },
      // normal
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 1,
        format: vk::Format::R32G32B32_SFLOAT,
        // offsetted by 'position' from beginning of structure
        offset: std::mem::size_of::<Vec3>() as u32,
      },
      // uv
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 2,
        format: vk::Format::R32G32_SFLOAT,
        // offsetted by 'position' and 'normal' from beginning of structure
        offset: 2 * std::mem::size_of::<Vec3>() as u32,
      },
    ]
  }
}
