use std::mem::size_of;

use ash::vk;
use glam::{vec3, Mat4, Vec3};

use crate::{
  config::Config,
  render_graph::TfxParamsUBO,
  vk_ctx::VkCtx,
  vk_utils::{VkBuffer, VkMemoryResource},
};

use super::{TfxFileData, TfxMaterial};

pub struct TfxObject {
  pub name: String,
  pub model_matrix: Mat4,
  pub center_of_gravity: Vec3,
  /// radius of each strand
  pub fiber_radius: f32,
  /// make strand tip thinner than the root by a factor e.g. half as thick
  pub thin_tip: f32,
  /// generate virtual/follow hairs based on each original/guide hair.
  /// Essentially, render each guide hair `followHairs` times with some displacement
  pub follow_hairs: u32,
  /// displacement of follow hair at the root
  pub follow_hair_spread_root: f32,
  /// displacement of follow hair at the tip
  pub follow_hair_spread_tip: f32,

  /// material
  pub material: TfxMaterial,

  /// Number of hair strands in this file. All strands in this file are guide strands.
  /// Follow hair strands are generated procedurally.
  ///
  /// **Sintel:** 228
  pub num_hair_strands: u32,

  /// From 4 to 64 inclusive (POW2 only). This should be a fixed value within tfx value.
  /// The total vertices from the tfx file is numHairStrands * numVerticesPerStrand.
  ///   
  /// **Sintel:** 32
  pub num_vertices_per_strand: u32,

  pub positions_buffer: VkBuffer,
  pub tangents_buffer: VkBuffer,
  pub index_buffer: VkBuffer,
  pub triangle_count: u32,

  /// Tfx params uploaded to GPU. Refreshed every frame (cause changes from ui etc.)
  pub tfx_params_ubo: Vec<VkBuffer>,
}

impl TfxObject {
  pub const MAX_FOLLOW_HAIRS_PER_GUIDE: u32 = 15;

  pub fn from_file(
    vk_ctx: &VkCtx,
    config: &Config,
    name: &str,
    model_matrix: Mat4,
    data: &TfxFileData,
  ) -> Self {
    let positions_buffer = create_positions_buffer(vk_ctx, &name, data);
    let tangents_buffer = create_tangents_buffer(vk_ctx, &name, data);
    let (index_buffer, triangle_count) = create_index_buffer(vk_ctx, &name, data);

    let tfx_params_ubo = allocate_params_ubo_vec(vk_ctx, name);

    let tfx_obj = Self {
      name: name.to_string(),
      model_matrix,
      center_of_gravity: vec3(0.0, 0.0, 0.0),
      material: TfxMaterial::default(),
      // tressfx:
      fiber_radius: 0.02,
      thin_tip: 0.9,
      follow_hairs: 10,
      follow_hair_spread_root: 0.3,
      follow_hair_spread_tip: 0.09,
      num_hair_strands: data.num_hair_strands,
      num_vertices_per_strand: data.num_vertices_per_strand,
      // buffers:
      positions_buffer,
      tangents_buffer,
      index_buffer,
      triangle_count, // closely related to `indices_buffer`
      tfx_params_ubo,
    };

    // write initial value to each buffer. Used if we rely on data from previous frame
    for i in 0..(tfx_obj.tfx_params_ubo.len()) {
      tfx_obj.update_params_uniform_buffer(i, config);
    }

    tfx_obj
  }

  pub unsafe fn destroy(&mut self, allocator: &vma::Allocator) {
    self.positions_buffer.delete(allocator);
    self.tangents_buffer.delete(allocator);
    self.index_buffer.delete(allocator);
    self.tfx_params_ubo.iter_mut().for_each(|buffer| {
      buffer.unmap_memory(allocator);
      buffer.delete(allocator);
    })
  }

  pub fn get_tfx_params_ubo_buffer(&self, frame_id: usize) -> &VkBuffer {
    &self.tfx_params_ubo[frame_id]
  }

  pub unsafe fn cmd_draw_mesh(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
    device.cmd_bind_index_buffer(
      command_buffer,
      self.index_buffer.buffer,
      0,
      vk::IndexType::UINT32,
    );

    let index_count = self.triangle_count * 3;
    let instance_count = self.follow_hairs;
    device.cmd_draw_indexed(command_buffer, index_count, instance_count, 0, 0, 0);
  }

  pub fn update_params_uniform_buffer(&self, frame_id: usize, config: &Config) {
    let data = TfxParamsUBO::new(config, self);
    let data_bytes = bytemuck::bytes_of(&data);
    let buffer = self.get_tfx_params_ubo_buffer(frame_id);
    buffer.write_to_mapped(data_bytes);
  }
}

fn create_positions_buffer(vk_ctx: &VkCtx, name: &str, data: &TfxFileData) -> VkBuffer {
  create_buffer_from_float_vec(
    vk_ctx,
    format!("{}.tfx_positions", name),
    &data.raw_vertex_positions,
  )
}

fn create_tangents_buffer(vk_ctx: &VkCtx, name: &str, data: &TfxFileData) -> VkBuffer {
  let total_float_cnt = (data.total_vertices() * 4) as usize;
  let mut tangents = Vec::<f32>::with_capacity(total_float_cnt);
  for _ in 0..total_float_cnt {
    tangents.push(0.0);
  }

  let mut set_tangent = |idx: usize, t: Vec3| {
    tangents[idx * 4 + 0] = t[0];
    tangents[idx * 4 + 1] = t[1];
    tangents[idx * 4 + 2] = t[2];
    tangents[idx * 4 + 3] = 0.0;
  };

  for i_strand in 0..data.num_hair_strands {
    // index of the first vertex of this strand
    let first_vert_idx: usize = (i_strand * data.num_vertices_per_strand) as _;
    let vert_0 = data.get_vertex_pos(first_vert_idx);
    let vert_1 = data.get_vertex_pos(first_vert_idx + 1);

    let tangent = subtract_norm(vert_1, vert_0);
    set_tangent(first_vert_idx, tangent);

    // vertex 1 through n-1
    for i in 1..data.num_vertices_per_strand {
      let ii = i as usize;
      let vert_i_minus_1 = data.get_vertex_pos(first_vert_idx + ii - 1);
      let vert_i = data.get_vertex_pos(first_vert_idx + ii);

      // Tangent for tips (last vert of strand).
      // Should not use tangent_next as it's a vert from next strand
      let tangent_pre = subtract_norm(vert_i, vert_i_minus_1);
      let mut tangent = tangent_pre;

      // Tangent for verts between first and last strand's verts.
      // Avg of tangent_pre, tangent_next
      if i != data.num_vertices_per_strand - 1 {
        let vert_i_plus_1 = data.get_vertex_pos(first_vert_idx + ii + 1);
        let tangent_next = subtract_norm(vert_i_plus_1, vert_i);
        tangent = add_norm(tangent_pre, tangent_next);
      }

      set_tangent(first_vert_idx + ii, tangent);
    }
  }

  create_buffer_from_float_vec(vk_ctx, format!("{}.tfx_tangents", name), &tangents)
}

fn subtract_norm(a: Vec3, b: Vec3) -> Vec3 {
  (a - b).normalize()
}

fn add_norm(a: Vec3, b: Vec3) -> Vec3 {
  (a + b).normalize()
}

fn create_index_buffer(vk_ctx: &VkCtx, name: &str, data: &TfxFileData) -> (VkBuffer, u32) {
  let count = data.total_vertices() * 6;
  let mut idx_data = Vec::<u32>::with_capacity(count as _);
  for _ in 0..count {
    idx_data.push(0);
  }

  let mut id = 0;
  let mut vert_idx = 0;

  for _ in 0..data.num_hair_strands {
    for _ in 0..(data.num_vertices_per_strand - 1) {
      // triangle 1
      idx_data[vert_idx] = 2 * id;
      vert_idx += 1;
      idx_data[vert_idx] = 2 * id + 1;
      vert_idx += 1;
      idx_data[vert_idx] = 2 * id + 2;
      vert_idx += 1;
      // triangle 2
      idx_data[vert_idx] = 2 * id + 2;
      vert_idx += 1;
      idx_data[vert_idx] = 2 * id + 1;
      vert_idx += 1;
      idx_data[vert_idx] = 2 * id + 3;
      vert_idx += 1;
      id += 1;
    }
    id += 1;
  }

  let bytes = bytemuck::cast_slice(&idx_data);
  let buffer = vk_ctx.create_buffer_from_data(
    format!("{}.tfx_tangents", name),
    bytes,
    vk::BufferUsageFlags::INDEX_BUFFER,
  );
  let triangle_cnt: u32 = (vert_idx / 3) as _;
  (buffer, triangle_cnt)
}

fn create_buffer_from_float_vec(vk_ctx: &VkCtx, name: String, data: &Vec<f32>) -> VkBuffer {
  let bytes = bytemuck::cast_slice(&data);
  vk_ctx.create_buffer_from_data(name, bytes, vk::BufferUsageFlags::STORAGE_BUFFER)
}

fn allocate_params_ubo(vk_ctx: &VkCtx, name: &str, frame_idx: usize) -> VkBuffer {
  let size = size_of::<TfxParamsUBO>() as _;
  vk_ctx.create_buffer_empty(
    format!("{}.params_ubo#{}", name, frame_idx),
    size,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
    true,
  )
}

pub fn allocate_params_ubo_vec(vk_ctx: &VkCtx, name: &str) -> Vec<VkBuffer> {
  let in_flight_frames = vk_ctx.frames_in_flight();
  (0..in_flight_frames)
    .map(|i| allocate_params_ubo(vk_ctx, &name, i))
    .collect::<Vec<_>>()
}
