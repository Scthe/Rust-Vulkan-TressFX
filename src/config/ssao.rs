pub struct SSAOConfig {
  /// - 2 - half-res
  /// - 4 - quater-res
  pub texture_size_div: u32,
  pub kernel_size: u32,
  pub radius: f32,
  pub bias: f32,
  pub blur_radius: usize,
  pub blur_gauss_sigma: f32,
  pub blur_max_depth_distance: f32,
  /// only meshes
  pub ao_strength: f32,
  /// only meshes
  pub ao_exp: f32,
}

impl SSAOConfig {
  pub const RNG_VECTOR_TEXTURE_SIZE: u32 = 4;
  pub const MAX_KERNEL_VALUES: u32 = 256;
}

impl Default for SSAOConfig {
  fn default() -> Self {
    Self {
      texture_size_div: 2,
      kernel_size: 24,
      radius: 0.5,
      bias: 0.025,
      blur_radius: 7,
      blur_gauss_sigma: 3.0,
      blur_max_depth_distance: 0.06,
      ao_strength: 0.3,
      ao_exp: 3.0,
    }
  }
}

/*
  // <editor-fold> SSAO
  public readonly ssao = {
    textureSizeMul: 0.5, // half/quater-res, wrt. MSAA
    kernelSize: 24,
    radius: 0.5,
    bias: 0.025,
    blurRadius: 7.0,
    blurGaussSigma: 3.0,
    blurMaxDepthDistance: 0.06,
    aoStrength: 0.3, // only meshes
    aoExp: 3, // only meshes
  };
  // </editor-fold> // END: SSAO
*/
