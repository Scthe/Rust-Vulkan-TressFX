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
    TriangleVertex::new((-0.5, -0.5), (1.0, 0.0, 0.0)), // 0, BL, red
    TriangleVertex::new((0.5, -0.5), (0.0, 0.0, 1.0)),  // 2, BR, blue
    TriangleVertex::new((0.5, 0.5), (1.0, 1.0, 1.0)),   // 3, TR, white
    TriangleVertex::new((-0.5, 0.5), (0.0, 1.0, 0.0)),  // 1, TL, green
  ];
  info!("Triangle vertex buffer: {} vertices", vertices.len());
  // allocate
  let vertices_bytes = bytemuck::cast_slice(&vertices);
  let vertex_buffer = VkBuffer::from_data(
    vertices_bytes,
    vk::BufferUsageFlags::VERTEX_BUFFER,
    &vk_ctx.allocator,
    vk_ctx.device.queue_family_index,
  );

  // index buffer
  let indices = [
    0u32, 1u32, 2u32, //
    2u32, 3u32, 0u32, //
  ];
  info!(
    "Triangle index buffer: {} indices, {} triangles",
    indices.len(),
    indices.len() / 3
  );
  let indices_bytes = bytemuck::cast_slice(&indices);
  let index_buffer = VkBuffer::from_data(
    indices_bytes,
    vk::BufferUsageFlags::INDEX_BUFFER,
    &vk_ctx.allocator,
    vk_ctx.device.queue_family_index,
  );

  WorldEntity {
    name: String::from("DebugTriangles"),
    vertex_buffer,
    index_buffer,
    vertex_count: indices.len() as u32,
  }
}
