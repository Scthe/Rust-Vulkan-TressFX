use ash;
use ash::vk;
use glam::Mat4;

use crate::{
  render_graph::{
    ForwardModelUBO, FLAG_IS_METALIC, FLAG_USE_HAIR_SHADOW_TEXTURE, FLAG_USE_SPECULAR_TEXTURE,
  },
  vk_ctx::VkCtx,
  vk_utils::{VkBuffer, VkTexture},
};

use super::Material;

pub struct WorldEntity {
  pub name: String,
  pub model_matrix: Mat4,

  // mesh
  pub vertex_buffer: VkBuffer,
  pub index_buffer: VkBuffer,
  pub vertex_count: u32,
  /// material+textures
  pub material: Material,
  /// For static data that does not change, see `ForwardModelUBO`
  pub model_constants_ubo: VkBuffer,
  // For data that changes per-frame, see `ForwardModelPerFrameUBO`
  // pub model_pre_frame_ubo: Vec<VkBuffer>,
}

impl WorldEntity {
  pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    self.vertex_buffer.delete(allocator);
    self.index_buffer.delete(allocator);
    self.material.destroy(device, allocator);
    self.model_constants_ubo.delete(allocator);
    // self.model_pre_frame_ubo.iter_mut().for_each(|buffer| {
    // buffer.unmap_memory(allocator);
    // buffer.delete(allocator);
    // })
  }

  /*pub fn create_per_frame_ubo_data(&self, camera: Camera) -> ForwardModelPerFrameUBO {
    ForwardModelPerFrameUBO {
      u_mvp: camera.model_view_projection_matrix(self.model_matrix),
    }
  }*/

  pub fn get_specular_texture(&self) -> &VkTexture {
    match &self.material.specular_tex {
      Some(tex) => &tex,
      None => &self.material.albedo_tex,
    }
  }
}

fn flag_bits(cond: bool, bit: i32) -> i32 {
  if cond {
    bit
  } else {
    0
  }
}

fn create_constants_ubo(material: &Material, model_matrix: Mat4) -> ForwardModelUBO {
  let mut material_flags: i32 = 0;
  material_flags |= flag_bits(material.is_metallic, FLAG_IS_METALIC);
  material_flags |= flag_bits(material.specular_tex.is_some(), FLAG_USE_SPECULAR_TEXTURE);
  material_flags |= flag_bits(
    material.hair_shadow_tex.is_some(),
    FLAG_USE_HAIR_SHADOW_TEXTURE,
  );

  ForwardModelUBO {
    u_model_matrix: model_matrix,
    u_specular: material.specular,
    u_specular_mul: material.specular_mul,
    u_material_flags: material_flags,
    u_sss_transluency: material.sss_transluency,
    u_sss_width: material.sss_width,
    u_sss_bias: material.sss_bias,
    u_sss_gain: material.sss_gain,
    u_sss_strength: material.sss_strength,
  }
}

pub fn allocate_constants_ubo(
  vk_ctx: &VkCtx,
  name: &str,
  material: &Material,
  model_matrix: Mat4,
) -> VkBuffer {
  let data = create_constants_ubo(material, model_matrix);
  let data_bytes = bytemuck::bytes_of(&data);

  VkBuffer::from_data(
    format!("{}_constants_ubo", name),
    data_bytes,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
    &vk_ctx.allocator,
    vk_ctx.device.queue_family_index,
  )
}

/*
fn allocate_per_frame_ubo(vk_ctx: &VkCtx, name: &str, frame_idx: usize) -> VkBuffer {
  let allocator = &vk_ctx.allocator;
  let size = size_of::<ForwardModelPerFrameUBO>() as _;

  let mut buffer = VkBuffer::empty(
    format!("{}_per_frame_ubo_#{}", name, frame_idx),
    size,
    vk::BufferUsageFlags::UNIFORM_BUFFER,
    allocator,
    vk_ctx.device.queue_family_index,
    true,
  );
  buffer.map_memory(allocator); // always mapped
  buffer
}

pub fn allocate_per_frame_ubo_vec(vk_ctx: &VkCtx, name: &str) -> Vec<VkBuffer> {
  let in_flight_frames = vk_ctx.frames_in_flight();
  (0..in_flight_frames)
    .map(|i| allocate_per_frame_ubo(vk_ctx, &name, i))
    .collect::<Vec<_>>()
}
*/
