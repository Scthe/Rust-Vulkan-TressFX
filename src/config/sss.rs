use super::ShadowSourceCfg;

pub struct SSSForwardScatterPassCfg {
  pub depthmap_size: u32,
  pub source: ShadowSourceCfg,
}

impl Default for SSSForwardScatterPassCfg {
  fn default() -> Self {
    Self {
      depthmap_size: 1024,
      source: ShadowSourceCfg {
        pos_phi: -93.0,  // horizontal [dgr]
        pos_theta: 55.0, // verical [dgr]
        ..ShadowSourceCfg::default()
      },
    }
  }
}

pub struct SSSBlurPassCfg {
  pub blur_width: f32,
  pub blur_strength: f32,
  /// slight changes for incident angles ~90dgr
  pub blur_follow_surface: bool,
}

impl Default for SSSBlurPassCfg {
  fn default() -> Self {
    Self {
      blur_width: 0.25,
      blur_strength: 0.03,
      blur_follow_surface: true,
    }
  }
}

impl SSSBlurPassCfg {
  pub const SSS_WIDTH_MIN: f32 = 0.01;
  pub const SSS_WIDTH_MAX: f32 = 1.0;
}
