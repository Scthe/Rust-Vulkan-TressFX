use std::path::Path;

use ash::vk;
use glam::Mat4;
use glam::Vec2;
use glam::Vec3;
use log::info;
use tobj;

use crate::render_graph::RenderableVertex;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

pub use self::camera::*;
pub use self::material::*;
pub use self::world::*;
pub use self::world_entity::*;

mod camera;
mod material;
mod world;
mod world_entity;

pub fn load_scene(vk_ctx: &VkCtx, cam_settings: CameraSettings) -> World {
  let scale = 0.8f32; // TODO from config
  let model_matrix = Mat4::from_scale(Vec3::new(scale, scale, scale));
  let sintel = load_sintel(vk_ctx, model_matrix);
  let sintel_eyes = load_sintel_eyes(vk_ctx, model_matrix);

  World {
    entities: vec![sintel, sintel_eyes],
    camera: Camera::new(cam_settings),
  }
}

fn load_sintel(vk_ctx: &VkCtx, model_matrix: Mat4) -> WorldEntity {
  let device = vk_ctx.vk_device();

  let tex_diffuse = VkTexture::from_file(
    device,
    &vk_ctx.allocator,
    vk_ctx,
    Path::new("./assets/sintel_lite_v2_1/textures/sintel_skin_diff.jpg"),
    vk::Format::R8G8B8A8_SRGB,
  );
  let specular_tex = VkTexture::from_file(
    device,
    &vk_ctx.allocator,
    vk_ctx,
    Path::new("./assets/sintel_lite_v2_1/textures/sintel_skin_spec.jpg"),
    VkTexture::RAW_DATA_TEXTURE_FORMAT,
  );
  let hair_shadow_tex = VkTexture::from_file(
    device,
    &vk_ctx.allocator,
    vk_ctx,
    Path::new("./assets/sintel_lite_v2_1/textures/sintel_hair_shadow.jpg"),
    VkTexture::RAW_DATA_TEXTURE_FORMAT,
  );
  let material = Material::new(tex_diffuse, Some(specular_tex), Some(hair_shadow_tex));
  let mesh = load_obj_mesh(vk_ctx, Path::new("./assets/sintel_lite_v2_1/sintel.obj"));
  let name = "sintel".to_string();
  let model_ubo = allocate_model_ubo_vec(vk_ctx, &name);

  WorldEntity {
    name: "sintel".to_string(),
    material,
    vertex_buffer: mesh.vertex_buffer,
    index_buffer: mesh.index_buffer,
    vertex_count: mesh.vertex_count,
    model_matrix,
    model_ubo,
  }
}

fn load_sintel_eyes(vk_ctx: &VkCtx, model_matrix: Mat4) -> WorldEntity {
  let device = vk_ctx.vk_device();

  let tex_diffuse = VkTexture::from_file(
    device,
    &vk_ctx.allocator,
    vk_ctx,
    Path::new("./assets/sintel_lite_v2_1/textures/sintel_eyeball_diff.jpg"),
    vk::Format::R8G8B8A8_SRGB,
  );

  let mut material = Material::new(tex_diffuse, None, None);
  material.specular_mul = 3.0; // shiny!
  let mesh = load_obj_mesh(
    vk_ctx,
    Path::new("./assets/sintel_lite_v2_1/sintel_eyeballs.obj"),
  );
  let name = "sintel".to_string();
  let model_ubo = allocate_model_ubo_vec(vk_ctx, &name);

  WorldEntity {
    name,
    material,
    vertex_buffer: mesh.vertex_buffer,
    index_buffer: mesh.index_buffer,
    vertex_count: mesh.vertex_count,
    model_matrix,
    model_ubo,
  }
}

struct Mesh {
  pub vertex_buffer: VkBuffer,
  pub index_buffer: VkBuffer,
  pub vertex_count: u32,
}

fn load_obj_mesh(vk_ctx: &VkCtx, path: &std::path::Path) -> Mesh {
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

  Mesh {
    vertex_buffer,
    index_buffer,
    vertex_count: indices.len() as u32,
  }
}
