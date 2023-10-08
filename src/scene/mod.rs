use std::path::Path;

use ash::vk;
use glam::Vec2;
use glam::Vec3;
use log::info;
use tobj;

use crate::render_graph::RenderableVertex;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

pub use self::camera::*;
pub use self::world::*;

mod camera;
mod world;

pub fn load_scene(vk_ctx: &VkCtx, cam_settings: CameraSettings) -> World {
  let cube_mesh = load_obj_mesh(vk_ctx, Path::new("./assets/cube.obj"));
  // panic!("as expected");

  // let debug_triangle = create_debug_triangles_scene(vk_ctx);

  World {
    // entities: vec![debug_triangle],
    entities: vec![cube_mesh],
    camera: Camera::new(cam_settings),
  }
}

fn create_debug_triangles_scene(vk_ctx: &VkCtx) -> WorldEntity {
  let uv = (0.0, 0.0);
  let vertices = [
    RenderableVertex::new((-0.5, -0.5, 0.0), (1.0, 0.0, 0.0), uv), // 0, BL, red
    RenderableVertex::new((0.5, -0.5, 0.0), (0.0, 0.0, 1.0), uv),  // 2, BR, blue
    RenderableVertex::new((0.5, 0.5, 0.0), (1.0, 1.0, 1.0), uv),   // 3, TR, white
    RenderableVertex::new((-0.5, 0.5, 0.0), (0.0, 1.0, 0.0), uv),  // 1, TL, green
  ];
  info!("Triangle vertex buffer: {} vertices", vertices.len());
  // allocate
  let vertices_bytes = bytemuck::cast_slice(&vertices);
  let vertex_buffer = VkBuffer::from_data(
    "TriangleVertexBuffer".to_string(),
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
    "TriangleIndexBuffer".to_string(),
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

fn load_obj_mesh(vk_ctx: &VkCtx, path: &std::path::Path) -> WorldEntity {
  let (models, _) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)
    .expect(&format!("Failed to load OBJ file '{}'", path.display()));

  if models.len() != 1 {
    panic!(
      "Expected 1 model in OBJ file, found {}. File: '{}'",
      models.len(),
      path.display()
    )
  }

  let object = &models[0];
  let mesh = &object.mesh;
  info!(
    "Loading OBJ file '{}'. Found object '{}' with {} vertices, {} normals, {} uv coordinates and {} triangles",
    path.display(),
    object.name,
    mesh.positions.len() / 3,
    mesh.normals.len() / 3,
    mesh.texcoords.len() / 2,
    mesh.indices.len() /3
  );

  let vertex_count = mesh.positions.len() / 3;
  let mut vertices: Vec<RenderableVertex> = Vec::with_capacity(vertex_count);
  let m_ps = &mesh.positions;
  let m_n = &mesh.normals;
  let m_uv = &mesh.texcoords;

  (0..vertex_count).for_each(|i| {
    let i3 = i * 3;
    let i2 = i * 2;
    vertices.push(RenderableVertex {
      position: Vec3::new(m_ps[i3], m_ps[i3 + 1], m_ps[i3 + 2]),
      normal: Vec3::new(m_n[i3], m_n[i3 + 1], m_n[i3 + 2]),
      uv: Vec2::new(m_uv[i2], m_uv[i2 + 1]),
    })
  });

  // allocate
  let vertices_bytes = bytemuck::cast_slice(&vertices);
  let vertex_buffer = VkBuffer::from_data(
    format!("{}_VertexBuffer", object.name),
    vertices_bytes,
    vk::BufferUsageFlags::VERTEX_BUFFER,
    &vk_ctx.allocator,
    vk_ctx.device.queue_family_index,
  );

  // index buffer
  let indices = &mesh.indices;
  let indices_bytes = bytemuck::cast_slice(&indices);
  let index_buffer = VkBuffer::from_data(
    format!("{}_IndexBuffer", object.name),
    indices_bytes,
    vk::BufferUsageFlags::INDEX_BUFFER,
    &vk_ctx.allocator,
    vk_ctx.device.queue_family_index,
  );

  WorldEntity {
    name: object.name.clone(),
    vertex_buffer,
    index_buffer,
    vertex_count: indices.len() as u32,
  }
}
