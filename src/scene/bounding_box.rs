use std::fmt;

use glam::{vec3, vec4, Mat4, Vec3};

use crate::render_graph::RenderableVertex;

/// More for debug and info than for actuall use
pub struct BoundingBox {
  /// In world space
  pub min: Vec3,
  /// In world space
  pub max: Vec3,
}

impl BoundingBox {
  pub fn from_vertices(vertices: &Vec<RenderableVertex>, model_matrix: Mat4) -> Self {
    assert!(vertices.len() > 0, "Mesh does not contain vertices");

    let mut bb_min = vertices[0].position.clone();
    let mut bb_max = vertices[0].position.clone();
    vertices.iter().for_each(|vert| {
      bb_min = bb_min.min(vert.position);
      bb_max = bb_max.max(vert.position);
    });
    let min = model_matrix * vec4(bb_min.x, bb_min.y, bb_min.z, 1.0);
    let max = model_matrix * vec4(bb_max.x, bb_max.y, bb_max.z, 1.0);

    Self {
      min: vec3(min.x, min.y, min.z),
      max: vec3(max.x, max.y, max.z),
    }
  }

  pub fn center(&self) -> Vec3 {
    (self.min + self.max) / 2.0
  }

  pub fn dimensions(&self) -> Vec3 {
    (self.min - self.max).abs()
  }
}

impl fmt::Display for BoundingBox {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "center={}, bounding box = (min={}, max={})",
      self.min,
      self.max,
      self.center()
    )
  }
}
