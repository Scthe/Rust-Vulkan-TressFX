use super::{Camera, WorldEntity};

pub struct World {
  pub camera: Camera,
  pub entities: Vec<WorldEntity>,
}

impl World {
  pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    for entity in &mut self.entities {
      entity.destroy(device, allocator);
    }
  }
}
