use crate::vk_utils::VkTexture;

pub struct Material {
  pub is_metallic: bool,
  pub specular: f32,
  /// Not PBR, but needed for eyes
  pub specular_mul: f32,
  // SSS forward (transluency)
  pub sss_transluency: f32,
  pub sss_width: f32,
  pub sss_bias: f32,
  pub sss_gain: f32,
  pub sss_strength: f32,
  /// Albedo for dielectrics, F0 for metalics
  pub albedo_tex: VkTexture,
  /// The usual specular texture
  pub specular_tex: Option<VkTexture>,
  /// Special texture for this demo
  pub hair_shadow_tex: Option<VkTexture>,
}

impl Material {
  pub fn new(
    albedo_tex: VkTexture,
    specular_tex: Option<VkTexture>,
    hair_shadow_tex: Option<VkTexture>,
  ) -> Material {
    Material {
      is_metallic: false,
      specular: 0.7,
      specular_mul: 1.0,
      sss_transluency: 0.5,
      sss_width: 60.0,
      sss_bias: 0.022,
      sss_gain: 0.0,
      sss_strength: 1.0,
      albedo_tex,
      specular_tex,
      hair_shadow_tex,
    }
  }
}

impl Material {
  pub unsafe fn destroy(&mut self, device: &ash::Device, allocator: &vma::Allocator) -> () {
    self.albedo_tex.delete(device, allocator);
    if let Some(mut tex) = self.specular_tex.take() {
      tex.delete(device, allocator);
    }
    if let Some(mut tex) = self.hair_shadow_tex.take() {
      tex.delete(device, allocator);
    }
  }
}
