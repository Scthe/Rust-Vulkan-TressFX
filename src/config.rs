use ash;
use ash::vk;
use glam::{vec3, Vec3};

use crate::utils::color_hex_to_vec;

pub use self::{camera::*, color_grading::*, light::*, postfx::*, shadows::*, ssao::*};

pub mod camera;
pub mod color_grading;
pub mod light;
pub mod postfx;
pub mod shadows;
pub mod ssao;

// Must match consts in `present.frag.glsl`.
pub enum DisplayMode {
  Final = 0,
  Normals = 1,
  Luma = 2,
  SSAO = 3,
}

/// https://github.com/Scthe/WebFX/blob/master/src/Config.ts
pub struct Config {
  /// crash program after first frame to read init errors
  pub only_first_frame: bool,
  /// debug display mode
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
  // ssao
  pub ssao: SSAOConfig,
  // postfx
  pub postfx: PostFxCfg,
  // misc
  // showDebugPositions = false;
  // useMSAA = true; // ok, technically it's brute force supersampling, but who cares?
  // center_of_gravity: vec3(0, 3.0, 0), // used for calulating hair normals (remember, no cards!)
}

impl Config {
  // public readonly stencilConsts = {
  // skin: 1 << 0,
  // hair: 1 << 1,
  // };

  pub fn new() -> Config {
    let clear_col: u8 = 93;

    Config {
      only_first_frame: false,
      display_mode: DisplayMode::Final as _,
      // window
      window_width: 800f64,
      window_height: 600f64,
      // clear colors
      clear_color: color_hex_to_vec(clear_col, clear_col, clear_col),
      clear_normal: vec3(0.0, 0.0, 0.0),
      clear_depth: 1.0,
      clear_stencil: 0,
      // scene
      model_scale: 1.0, // TODO it was 0.3 in webfx?
      camera: CameraConfig::default(),
      light_ambient: LightAmbient::default(),
      light0: LightCfg::light0(),
      light1: LightCfg::light1(),
      light2: LightCfg::light2(),
      // postfx
      ssao: SSAOConfig::default(),
      shadows: ShadowsConfig::default(),
      postfx: PostFxCfg::default(),
    }
  }

  pub fn get_ssao_viewport_size(&self) -> vk::Extent2D {
    vk::Extent2D {
      width: (self.window_width as u32) / self.ssao.texture_size_div,
      height: (self.window_height as u32) / self.ssao.texture_size_div,
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
