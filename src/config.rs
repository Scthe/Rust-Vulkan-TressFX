use ash;
use ash::vk;
use glam::{vec2, vec3, Vec2, Vec3};
use mint::Vector3;

pub struct LightAmbient {
  pub color: Vec3,
  pub energy: f32,
}

pub struct LightCfg {
  /// horizontal [dgr]
  pub pos_phi: f32,
  /// verical [dgr]
  pub pos_theta: f32,
  pub pos_distance: f32,
  pub color: Vec3,
  pub energy: f32,
}

pub struct ShadowsConfig {
  pub strength: f32,
}

pub struct CameraConfig {
  pub position: Vec3,
  pub rotation: Vec2,
  pub fov_dgr: f32,
  pub z_near: f32,
  pub z_far: f32,
}

pub enum TonemappingMode {
  Linear = 0,
  Reinhard = 1,
  Uncharted2 = 2,
  Photographic = 3,
  AcesUe4 = 4,
}

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

pub struct PostFxCfg {
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

/// https://github.com/Scthe/WebFX/blob/master/src/Config.ts
pub struct Config {
  /// crash program after first frame to read init errors
  pub only_first_frame: bool,
  /// debug display mode, see UISystem for modes
  pub display_mode: usize,
  // window
  pub window_width: f64,
  pub window_height: f64,
  // clear colors
  pub clear_color: Vec3,
  pub clear_normal: Vec3,
  pub clear_depth: f32,
  pub clear_stencil: i8,
  // scene-related
  pub model_scale: f32,
  pub camera: CameraConfig,
  // lights
  pub light_ambient: LightAmbient,
  pub light0: LightCfg,
  pub light1: LightCfg,
  pub light2: LightCfg,
  // shadows
  pub shadows: ShadowsConfig,
  // postfx
  pub postfx: PostFxCfg,
  // misc
  // showDebugPositions = false;
  // useMSAA = true; // ok, technically it's brute force supersampling, but who cares?
  // center_of_gravity: vec3(0, 3.0, 0), // used for calulating hair normals (remember, no cards!)
}

impl Config {
  // Must match consts in `present.frag.glsl`. TODO switch to enum?
  // TODO add luma/raw render debug
  pub const DISPLAY_MODE_FINAL: usize = 0;
  pub const DISPLAY_MODE_NORMALS: usize = 1;

  // public readonly stencilConsts = {
  // skin: 1 << 0,
  // hair: 1 << 1,
  // };

  pub fn new() -> Config {
    let clear_col = to_col8(93);

    let camera = CameraConfig {
      position: vec3(4.0, 7.5, 9.0),
      // position: vec3(0.0, 2.5, 5.0), // en face
      // position: vec3(0, 3.5, 2), // closeup on hair
      rotation: vec2(-25f32, 0.0), // degrees
      // rotation: vec2(0.0, 0.0), // degrees
      fov_dgr: 75.0,
      z_near: 0.1,
      z_far: 100.0,
    };

    let light_ambient = LightAmbient {
      color: vec3(to_col8(160), to_col8(160), to_col8(160)),
      energy: 0.02,
    };
    let light0 = LightCfg {
      pos_phi: 125.0,  // horizontal [dgr]
      pos_theta: 45.0, // verical [dgr]
      pos_distance: 10.0,
      color: vec3(to_col8(214), to_col8(197), to_col8(208)),
      energy: 1.0,
    };
    let light1 = LightCfg {
      pos_phi: 45.0,   // horizontal [dgr]
      pos_theta: 82.0, // verical [dgr]
      pos_distance: 10.0,
      color: vec3(to_col8(214), to_col8(166), to_col8(166)),
      energy: 0.80,
    };
    let light2 = LightCfg {
      pos_phi: -105.0, // horizontal [dgr]
      pos_theta: 55.0, // verical [dgr]
      pos_distance: 10.0,
      color: vec3(to_col8(133), to_col8(171), to_col8(169)),
      energy: 0.55,
    };

    let postfx = PostFxCfg {
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
        global: ColorGradingPerRangeSettings::default(),
        shadows: ColorGradingPerRangeSettings::default(),
        midtones: ColorGradingPerRangeSettings::default(),
        highlights: ColorGradingPerRangeSettings::default(),
      },
      // fxaa
      use_fxaa: true, // TODO test how good it works
      subpixel: 0.75,
      edge_threshold: 0.125,
      edge_threshold_min: 0.0625,
      fxaa_luma_gamma: 2.2,
    };

    Config {
      only_first_frame: false,
      display_mode: Config::DISPLAY_MODE_FINAL,
      // window
      window_width: 800f64,
      window_height: 600f64,
      // clear colors
      clear_color: vec3(clear_col, clear_col, clear_col),
      clear_normal: vec3(0.0, 0.0, 0.0),
      clear_depth: 1.0,
      clear_stencil: 0,
      // scene
      model_scale: 1.0, // TODO it was 0.3 in webfx?
      camera,
      // lights
      light_ambient,
      light0,
      light1,
      light2,
      // shadows
      shadows: ShadowsConfig { strength: 0.7 },
      // postfx
      postfx,
    }
  }

  pub fn clear_color(&self) -> vk::ClearValue {
    let cc = self.clear_color;
    vk::ClearValue {
      color: vk::ClearColorValue {
        float32: [cc[0], cc[1], cc[2], 1f32],
      },
    }
  }

  pub fn clear_normals(&self) -> vk::ClearValue {
    let cc = self.clear_normal;
    vk::ClearValue {
      color: vk::ClearColorValue {
        float32: [cc[0], cc[1], cc[2], 1f32],
      },
    }
  }

  pub fn clear_depth_stencil(&self) -> vk::ClearValue {
    vk::ClearValue {
      depth_stencil: vk::ClearDepthStencilValue {
        depth: self.clear_depth,
        stencil: self.clear_stencil as u32,
      },
    }
  }

  pub fn fxaa_edge_threshold(&self) -> f32 {
    if self.postfx.use_fxaa {
      self.postfx.edge_threshold
    } else {
      0.0
    }
  }
}

fn to_col8(col_u8: u8) -> f32 {
  (col_u8 as f32) / 255.0
}

/// Convert spherical->cartesian. Both angles in degrees.
pub fn spherical_to_cartesian_dgr(phi_dgr: f32, theta_dgr: f32, distance: f32) -> Vec3 {
  spherical_to_cartesian_rad(phi_dgr.to_radians(), theta_dgr.to_radians(), distance)
}

/// Convert spherical->cartesian. Both angles in radians.
pub fn spherical_to_cartesian_rad(phi: f32, theta: f32, distance: f32) -> Vec3 {
  vec3(
    f32::cos(phi) * f32::sin(theta) * distance,
    f32::cos(theta) * distance,
    f32::sin(phi) * f32::sin(theta) * distance,
  )
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

/*
  public readonly lightSSS = {
    // forward scatter
    depthmapSize: 1024,
    posPhi: -93, // horizontal [dgr]
    posTheta: 55, // verical [dgr]
    posRadius: SHADOWS_ORTHO_SIZE,
    // SSS blur pass
    blurWidth: 25.0,
    blurStrength: 0.35,
    blurFollowSurface: false, // slight changes for incident angles ~90dgr
    // will reuse target & projection settings from shadows - safer this way..
  };
*/

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
