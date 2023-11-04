use std::path::Path;

use ash::vk;
use glam::Mat4;
use glam::Vec2;
use glam::Vec3;
use log::info;
use log::trace;
use tobj;

use crate::config::Config;
use crate::render_graph::RenderableVertex;
use crate::vk_ctx::VkCtx;
use crate::vk_utils::*;

pub use self::bounding_box::*;
pub use self::camera::*;
pub use self::material::*;
pub use self::tressfx::*;
pub use self::world::*;
pub use self::world_entity::*;

mod bounding_box;
mod camera;
mod material;
mod tressfx;
mod world;
mod world_entity;

pub fn load_scene(vk_ctx: &VkCtx, config: &Config) -> World {
  let scale = config.model_scale;
  let model_matrix = Mat4::from_scale(Vec3::new(scale, scale, scale));
  let sintel = load_sintel(vk_ctx, model_matrix);
  let sintel_eyes = load_sintel_eyes(vk_ctx, model_matrix);

  // tressfx
  let sintel_tfx_file = load_tressfx_file(std::path::Path::new(
    "./assets/sintel_lite_v2_1/GEO-sintel_hair_emit.002-sintel_hair.tfx",
  ));
  let sintel_hair = TfxObject::from_file(vk_ctx, "sintel_hair", model_matrix, &sintel_tfx_file);

  World {
    camera: Camera::new(config, vk_ctx.window_size()),
    entities: vec![sintel, sintel_eyes],
    tressfx_objects: vec![sintel_hair],
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
  let (mesh, aabb) = load_obj_mesh(
    vk_ctx,
    Path::new("./assets/sintel_lite_v2_1/sintel.obj"),
    &model_matrix,
  );
  let name = "sintel".to_string();
  let model_ubo = allocate_model_ubo_vec(vk_ctx, &name);

  WorldEntity {
    name,
    material,
    vertex_buffer: mesh.vertex_buffer,
    index_buffer: mesh.index_buffer,
    vertex_count: mesh.vertex_count,
    aabb,
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
  let (mesh, aabb) = load_obj_mesh(
    vk_ctx,
    Path::new("./assets/sintel_lite_v2_1/sintel_eyeballs.obj"),
    &model_matrix,
  );
  let name = "sintel_eyes".to_string();
  let model_ubo = allocate_model_ubo_vec(vk_ctx, &name);

  WorldEntity {
    name,
    material,
    vertex_buffer: mesh.vertex_buffer,
    index_buffer: mesh.index_buffer,
    vertex_count: mesh.vertex_count,
    aabb,
    model_matrix,
    model_ubo,
  }
}

struct Mesh {
  pub vertex_buffer: VkBuffer,
  pub index_buffer: VkBuffer,
  pub vertex_count: u32,
}

fn load_obj_mesh(
  vk_ctx: &VkCtx,
  path: &std::path::Path,
  model_matrix: &Mat4,
) -> (Mesh, BoundingBox) {
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
  let aabb = BoundingBox::from_vertices(&vertices, *model_matrix);
  trace!("{}", aabb);

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

  let mesh = Mesh {
    vertex_buffer,
    index_buffer,
    vertex_count: indices.len() as u32,
  };
  (mesh, aabb)
}
