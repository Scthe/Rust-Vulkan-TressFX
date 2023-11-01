use ash;
use ash::vk;
use glam::{vec2, Vec2, Vec3};

use crate::utils::color_hex_to_vec;

pub use self::{camera::*, color_grading::*, light::*, postfx::*, shadows::*, ssao::*, sss::*};

pub mod camera;
pub mod color_grading;
pub mod light;
pub mod postfx;
pub mod shadows;
pub mod ssao;
pub mod sss;

// Must match consts in `present.frag.glsl`.
pub enum DisplayMode {
  Final = 0,
  Normals = 1,
  Luma = 2,
  SSAO = 3,
  LinearDepth = 4, // TODO it's depth, not linear depth now?
  ShadowMap = 5,
  SSSContribution = 6,
  SSSThickness = 7,
}

/// https://github.com/Scthe/WebFX/blob/master/src/Config.ts
pub struct Config {
  /// crash program after first frame to read init errors
  pub only_first_frame: bool,
  /// show spheres where lights/shadows are
  pub show_debug_positions: bool,
  /// debug display mode
  pub display_mode: usize,
  pub linear_depth_preview_range: Vec2,
  // window
  pub window_width: f64,
  pub window_height: f64,
  // clear colors
  pub clear_color: Vec3,
  pub clear_normal: [u32; 4],
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
  // sss
  pub sss_forward_scatter: SSSForwardScatterPassCfg,
  pub sss_blur: SSSBlurPassCfg,
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
  pub const STENCIL_BIT_SKIN: u32 = 1 << 0;
  // pub const STENCIL_BIT_HAIR: u32 = 1 << 1;

  pub fn new() -> Config {
    let clear_col: u8 = 93;

    Config {
      only_first_frame: false,
      show_debug_positions: false,
      display_mode: DisplayMode::Final as _,
      linear_depth_preview_range: vec2(-2.0, -15.0),
      // window
      window_width: 800f64,
      window_height: 600f64,
      // clear colors
      clear_color: color_hex_to_vec(clear_col, clear_col, clear_col),
      clear_normal: [0, 0, 0, 0], // or [1,1,1,1] for performance reasons
      clear_depth: 1.0,
      clear_stencil: 0,
      // scene
      model_scale: 0.3,
      camera: CameraConfig::default(),
      light_ambient: LightAmbient::default(),
      light0: LightCfg::light0(),
      light1: LightCfg::light1(),
      light2: LightCfg::light2(),
      // material + lights
      ssao: SSAOConfig::default(),
      shadows: ShadowsConfig::default(),
      sss_forward_scatter: SSSForwardScatterPassCfg::default(),
      sss_blur: SSSBlurPassCfg::default(),
      // postfx
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
    vk::ClearValue {
      color: vk::ClearColorValue {
        uint32: self.clear_normal,
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

  /// Some debug display modes write to forward pass result.
  /// Avoid overriding them.
  pub fn preserve_original_forward_pass_result(&self) -> bool {
    let preserve_org_result = [
      DisplayMode::ShadowMap as usize,
      DisplayMode::SSSContribution as usize,
      DisplayMode::SSSThickness as usize,
    ];
    preserve_org_result.contains(&self.display_mode)
  }

  pub fn fxaa_edge_threshold(&self) -> f32 {
    if self.postfx.use_fxaa {
      self.postfx.edge_threshold
    } else {
      0.0
    }
  }

  pub fn get_camera_fov_y(&self) -> f32 {
    (self.camera.fov_dgr / self.window_width as f32) * self.window_height as f32
  }
}
