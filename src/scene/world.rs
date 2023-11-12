use super::{Camera, TfxObject, WorldEntity};

pub struct World {
  pub camera: Camera,
  pub entities: Vec<WorldEntity>,
  pub tressfx_objects: Vec<TfxObject>,
}

impl World {
  pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    for entity in &mut self.entities {
      entity.destroy(device, allocator);
    }

    for entity in &mut self.tressfx_objects {
      entity.destroy(allocator);
    }
  }
}
