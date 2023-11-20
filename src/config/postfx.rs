use super::color_grading::{ColorGradingCfg, ColorGradingPerRangeSettings};

pub enum TonemappingMode {
  Linear = 0,
  Reinhard = 1,
  Uncharted2 = 2,
  Photographic = 3,
  AcesUe4 = 4,
}

pub struct PostFxCfg {
  pub gamma: f32,
  pub dither_strength: f32,
  // tonemapping
  pub tonemapping_op: usize,
  pub exposure: f32, // or calc automatically?
  pub white_point: f32,
  pub aces_c: f32,
  pub aces_s: f32,
  // color grading
  // @see https://docs.unrealengine.com/en-us/Engine/Rendering/PostProcessEffects/ColorGrading
  pub color_grading: ColorGradingCfg,
  // fxaa
  pub use_fxaa: bool,
  pub fxaa_luma_gamma: f32,
  pub subpixel: f32,
  pub edge_threshold: f32,
  pub edge_threshold_min: f32,
}

impl Default for PostFxCfg {
  fn default() -> Self {
    let mut cg_global = ColorGradingPerRangeSettings::default();
    cg_global.gamma.value = 0.9;

    Self {
      gamma: 2.2,
      dither_strength: 1.5,
      // tonemapping
      tonemapping_op: TonemappingMode::AcesUe4 as _,
      exposure: 1.0, // or calc automatically?
      white_point: 1.0,
      aces_c: 0.8,
      aces_s: 1.0,
      // color grading
      // @see https://docs.unrealengine.com/en-us/Engine/Rendering/PostProcessEffects/ColorGrading
      color_grading: ColorGradingCfg {
        shadows_max: 0.09,
        highlights_min: 0.5,
        global: cg_global,
        shadows: ColorGradingPerRangeSettings::default(),
        midtones: ColorGradingPerRangeSettings::default(),
        highlights: ColorGradingPerRangeSettings::default(),
      },
      // fxaa
      use_fxaa: true,
      subpixel: 0.75,
      edge_threshold: 0.125,
      edge_threshold_min: 0.0625,
      fxaa_luma_gamma: 2.2,
    }
  }
}
