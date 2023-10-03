use ash::vk;
use log::info;

use crate::render_graph::TriangleVertex;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

pub use self::world::World;
pub use self::world::WorldEntity;

mod world;

pub fn load_scene(vk_ctx: &VkCtx) -> World {
  let debug_triangle = create_debug_triangles_scene(vk_ctx);
  World {
    entities: vec![debug_triangle],
  }
}

fn create_debug_triangles_scene(vk_ctx: &VkCtx) -> WorldEntity {
  let vertices = [
    TriangleVertex::new((0.0, 0.5), (1.0, 0.0, 0.0)), // red
    TriangleVertex::new((0.5, -0.5), (0.0, 1.0, 0.0)), // green
    TriangleVertex::new((-0.5, -0.5), (0.0, 0.0, 1.0)), // blue
    TriangleVertex::new((-0.4, 0.4), (1.0, 1.0, 1.0)), // white
  ];
  let vertices_bytes = bytemuck::cast_slice(&vertices);
  info!("Vertex buffer bytes={}", vertices_bytes.len());

  // allocate
  let vertex_buffer = VkBuffer::from_data(
    vertices_bytes,
    vk::BufferUsageFlags::VERTEX_BUFFER,
    &vk_ctx.allocator,
    vk_ctx.device.queue_family_index,
  );

  WorldEntity {
    name: String::from("DebugTriangles"),
    vertex_buffer,
    // index_buffer: VkBuffer,
    vertex_count: 3u32,
  }
}
