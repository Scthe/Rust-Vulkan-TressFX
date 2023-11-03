use std::fmt::Display;

use glam::{vec3, Vec3};

/// ### Offsets
/// Offset values are in bytes, aligned on 8 bytes boundaries,
/// and relative to beginning of the .tfx file. **ALL ARE UNSIGNED INTS!**
pub struct TfxFileData {
  /// Specifies TressFX version number
  pub version: f32,

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

  /// Array size: `FLOAT4[numHairStrands]`. **Sintel:** 160
  pub offset_vertex_position: u32,
  /// Array size: `FLOAT2[numHairStrands]`, if 0 no texture coordinates. **Sintel:** 0
  pub offset_strand_uv: u32,
  /// Array size: `FLOAT2[numHairStrands * numVerticesPerStrand]`, if 0, no per vertex texture coordinates. **Sintel:** 0
  pub offset_vertex_uv: u32,
  /// Array size: `float[numHairStrands]`
  pub offset_strand_thickness: u32,
  /// Array size: `FLOAT4[numHairStrands * numVerticesPerStrand]`, if 0, no vertex colors. **Sintel:** 0
  pub offset_vertex_color: u32,

  /// vertex positions - ready to shove onto GPU
  pub raw_vertex_positions: Vec<f32>,
}

impl TfxFileData {
  pub fn total_vertices(&self) -> u32 {
    self.num_vertices_per_strand * self.num_hair_strands
  }

  pub fn get_vertex_pos(&self, idx: usize) -> Vec3 {
    vec3(
      self.raw_vertex_positions[idx * 4],
      self.raw_vertex_positions[idx * 4 + 1],
      self.raw_vertex_positions[idx * 4 + 2],
      // self.raw_vertex_positions[idx * 4 + 3],
    )
  }
}

impl Display for TfxFileData {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "TfxFileData(")
      .and(write!(f, "version: {}", self.version))
      .and(write!(f, ", num_hair_strands: {}", self.num_hair_strands))
      .and(write!(
        f,
        ", num_vertices_per_strand: {}",
        self.num_vertices_per_strand
      ))
      .and(write!(
        f,
        ", offset_vertex_position: {}",
        self.offset_vertex_position
      ))
      .and(write!(f, ", offset_strand_uv: {}", self.offset_strand_uv))
      .and(write!(f, ", offset_vertex_uv: {}", self.offset_vertex_uv))
      .and(write!(
        f,
        ", offset_strand_thickness: {}",
        self.offset_strand_thickness
      ))
      .and(write!(
        f,
        ", offset_vertex_color: {}",
        self.offset_vertex_color
      ))
      .and(write!(f, ")"))
  }
}
