use glam::{vec3, Vec3};
use mint::Vector3;

pub struct ColorGradingProp {
  pub color: Vector3<f32>,
  pub value: f32,
}

impl ColorGradingProp {
  pub fn new(color: Vec3, value: f32) -> Self {
    Self {
      color: Vector3::from_slice(color.as_ref()),
      value,
    }
  }
}

pub struct ColorGradingPerRangeSettings {
  pub saturation: ColorGradingProp,
  pub contrast: ColorGradingProp,
  pub gamma: ColorGradingProp,
  pub gain: ColorGradingProp,
  pub offset: ColorGradingProp,
}

impl Default for ColorGradingPerRangeSettings {
  fn default() -> Self {
    Self {
      saturation: ColorGradingProp::new(vec3(1.0, 1.0, 1.0), 1.0),
      contrast: ColorGradingProp::new(vec3(1.0, 1.0, 1.0), 1.0),
      gamma: ColorGradingProp::new(vec3(1.0, 1.0, 1.0), 1.0),
      gain: ColorGradingProp::new(vec3(1.0, 1.0, 1.0), 1.0),
      offset: ColorGradingProp::new(vec3(0.0, 0.0, 0.0), 0.0),
    }
  }
}

pub struct ColorGradingCfg {
  pub global: ColorGradingPerRangeSettings,
  pub shadows: ColorGradingPerRangeSettings,
  pub midtones: ColorGradingPerRangeSettings,
  pub highlights: ColorGradingPerRangeSettings,
  // cutoffs:
  pub shadows_max: f32,
  pub highlights_min: f32,
}
