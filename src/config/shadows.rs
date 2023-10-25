pub struct ShadowsConfig {
  pub strength: f32,
}

impl Default for ShadowsConfig {
  fn default() -> Self {
    Self { strength: 0.7 }
  }
}

/*
const SHADOWS_ORTHO_SIZE = 5;

  public readonly shadows = {
    shadowmapSize: 1024 * 2,
    usePCSS: false,
    blurRadius: 4, // in pixels
    bias: 0.005,
    blurRadiusTfx: 1, // in pixels
    biasHairTfx: 0.050,
    hairTfxRadiusMultipler: 1.1,
    strength: 0.7,
    directionalLight: {
      posPhi: 105, // horizontal [dgr]
      posTheta: 45, // verical [dgr]
      posRadius: SHADOWS_ORTHO_SIZE, // verify with projection box below!!!
      target: Vec3(0, 2, 0),
      projection: {
        left: -SHADOWS_ORTHO_SIZE, right: SHADOWS_ORTHO_SIZE,
        top: SHADOWS_ORTHO_SIZE, bottom: -SHADOWS_ORTHO_SIZE,
        near: 0.1, far: 20,
      },
    },
    showDebugView: false,
  };
*/
