use ash;
use ash::vk;
use bytemuck;
use glam::Vec4;

/// Represents vertex layout that is used for each renderable object in the scene.
/// Rendered in `ForwardPass`
#[derive(Copy, Clone, Debug)] // , bytemuck::Zeroable, bytemuck::Pod
#[repr(C)]
pub struct RenderableVertex {
  pos: Vec4, // TODO Vec3, Vec2 are enough
  color: Vec4,
}
unsafe impl bytemuck::Zeroable for RenderableVertex {}
unsafe impl bytemuck::Pod for RenderableVertex {}

impl RenderableVertex {
  pub fn new(pos: (f32, f32), col: (f32, f32, f32)) -> RenderableVertex {
    RenderableVertex {
      pos: Vec4::new(pos.0, pos.1, 0.0f32, 1.0f32),
      color: Vec4::new(col.0, col.1, col.2, 1.0f32),
    }
  }

  pub fn get_bindings_descriptions() -> [vk::VertexInputBindingDescription; 1] {
    [vk::VertexInputBindingDescription {
      binding: 0,
      input_rate: vk::VertexInputRate::VERTEX,
      stride: std::mem::size_of::<RenderableVertex>() as u32,
    }]
  }

  pub fn get_attributes_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
    [
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 0,
        format: vk::Format::R32G32_SFLOAT,
        offset: 0, // offsetof(RenderableVertex, pos),
      },
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 1,
        format: vk::Format::R32G32B32_SFLOAT,
        offset: std::mem::size_of::<Vec4>() as u32, // offsetted by 'position' from beginning of structure
      },
    ]
  }
}
